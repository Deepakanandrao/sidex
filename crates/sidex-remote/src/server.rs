//! Lightweight SideX Server that runs on the remote machine.
//!
//! Exposes file operations, PTY management, command execution, and LSP
//! forwarding over a JSON-RPC protocol.  The transport is any
//! `AsyncRead + AsyncWrite` stream (SSH channel, WebSocket, stdin/stdout).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// JSON-RPC types (compatible with tunnel.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Request {
    #[allow(dead_code)]
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct Response {
    jsonrpc: &'static str,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i64,
    message: String,
}

impl Response {
    fn ok(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn err(id: u64, code: i64, msg: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: msg.into(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// PTY handle (server-side)
// ---------------------------------------------------------------------------

struct PtyHandle {
    child: tokio::process::Child,
}

// ---------------------------------------------------------------------------
// SideX Server
// ---------------------------------------------------------------------------

/// The remote server that handles JSON-RPC requests from the SideX client.
pub struct SideXServer {
    ptys: Arc<Mutex<HashMap<u64, PtyHandle>>>,
    next_pty_id: Arc<Mutex<u64>>,
}

impl Default for SideXServer {
    fn default() -> Self {
        Self::new()
    }
}

impl SideXServer {
    /// Create a new server instance.
    pub fn new() -> Self {
        Self {
            ptys: Arc::new(Mutex::new(HashMap::new())),
            next_pty_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Run the server loop, reading JSON-RPC requests from `reader` and
    /// writing responses to `writer`.
    ///
    /// Each line is a complete JSON-RPC message (newline-delimited JSON).
    pub async fn run<R, W>(&self, reader: R, writer: W) -> Result<()>
    where
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut lines = BufReader::new(reader).lines();
        let writer = Arc::new(Mutex::new(writer));

        while let Some(line) = lines.next_line().await? {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            let req: Request = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let resp = Response::err(0, -32700, format!("parse error: {e}"));
                    Self::send(&writer, &resp).await?;
                    continue;
                }
            };

            let resp = self.handle(req).await;
            Self::send(&writer, &resp).await?;
        }

        Ok(())
    }

    async fn send<W: AsyncWrite + Unpin + Send>(
        writer: &Arc<Mutex<W>>,
        resp: &Response,
    ) -> Result<()> {
        let mut json = serde_json::to_vec(resp)?;
        json.push(b'\n');
        let mut w = writer.lock().await;
        w.write_all(&json).await?;
        w.flush().await?;
        Ok(())
    }

    async fn handle(&self, req: Request) -> Response {
        match req.method.as_str() {
            "fs/readFile" => self.fs_read_file(req.id, &req.params).await,
            "fs/writeFile" => self.fs_write_file(req.id, &req.params).await,
            "fs/readDir" => self.fs_read_dir(req.id, &req.params).await,
            "fs/stat" => self.fs_stat(req.id, &req.params).await,
            "exec/run" => self.exec_run(req.id, &req.params).await,
            "pty/open" => self.pty_open(req.id, &req.params).await,
            "pty/write" => self.pty_write(req.id, &req.params).await,
            "pty/resize" => self.pty_resize(req.id, &req.params).await,
            _ => Response::err(req.id, -32601, format!("unknown method: {}", req.method)),
        }
    }

    // -- fs handlers --------------------------------------------------------

    async fn fs_read_file(&self, id: u64, params: &serde_json::Value) -> Response {
        let Some(path) = params.get("path").and_then(|v| v.as_str()) else {
            return Response::err(id, -32602, "missing `path` param");
        };
        match tokio::fs::read(path).await {
            Ok(data) => {
                let encoded = base64_encode(&data);
                Response::ok(id, serde_json::json!({ "data": encoded }))
            }
            Err(e) => Response::err(id, 1, e.to_string()),
        }
    }

    async fn fs_write_file(&self, id: u64, params: &serde_json::Value) -> Response {
        let Some(path) = params.get("path").and_then(|v| v.as_str()) else {
            return Response::err(id, -32602, "missing `path` param");
        };
        let Some(data_b64) = params.get("data").and_then(|v| v.as_str()) else {
            return Response::err(id, -32602, "missing `data` param");
        };
        let Ok(data) = base64_decode(data_b64) else {
            return Response::err(id, -32602, "invalid base64 data");
        };
        match tokio::fs::write(path, &data).await {
            Ok(()) => Response::ok(id, serde_json::json!({})),
            Err(e) => Response::err(id, 1, e.to_string()),
        }
    }

    async fn fs_read_dir(&self, id: u64, params: &serde_json::Value) -> Response {
        let Some(path) = params.get("path").and_then(|v| v.as_str()) else {
            return Response::err(id, -32602, "missing `path` param");
        };
        let mut entries = Vec::new();
        match tokio::fs::read_dir(path).await {
            Ok(mut dir) => {
                while let Ok(Some(entry)) = dir.next_entry().await {
                    let meta = entry.metadata().await.ok();
                    entries.push(serde_json::json!({
                        "name": entry.file_name().to_string_lossy(),
                        "path": entry.path().to_string_lossy(),
                        "is_dir": meta.as_ref().map_or(false, |m| m.is_dir()),
                        "size": meta.as_ref().map_or(0, |m| m.len()),
                    }));
                }
                Response::ok(id, serde_json::json!({ "entries": entries }))
            }
            Err(e) => Response::err(id, 1, e.to_string()),
        }
    }

    async fn fs_stat(&self, id: u64, params: &serde_json::Value) -> Response {
        let Some(path) = params.get("path").and_then(|v| v.as_str()) else {
            return Response::err(id, -32602, "missing `path` param");
        };
        match tokio::fs::symlink_metadata(path).await {
            Ok(meta) => Response::ok(
                id,
                serde_json::json!({
                    "size": meta.len(),
                    "is_dir": meta.is_dir(),
                    "is_symlink": meta.is_symlink(),
                }),
            ),
            Err(e) => Response::err(id, 1, e.to_string()),
        }
    }

    // -- exec handler -------------------------------------------------------

    async fn exec_run(&self, id: u64, params: &serde_json::Value) -> Response {
        let Some(command) = params.get("command").and_then(|v| v.as_str()) else {
            return Response::err(id, -32602, "missing `command` param");
        };
        match Command::new("sh").arg("-c").arg(command).output().await {
            Ok(out) => Response::ok(
                id,
                serde_json::json!({
                    "stdout": String::from_utf8_lossy(&out.stdout),
                    "stderr": String::from_utf8_lossy(&out.stderr),
                    "exit_code": out.status.code().unwrap_or(-1),
                }),
            ),
            Err(e) => Response::err(id, 1, e.to_string()),
        }
    }

    // -- pty handlers -------------------------------------------------------

    async fn pty_open(&self, id: u64, params: &serde_json::Value) -> Response {
        let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
        let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;

        let child = match Command::new("sh")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => return Response::err(id, 1, e.to_string()),
        };

        let mut next = self.next_pty_id.lock().await;
        let pty_id = *next;
        *next += 1;

        self.ptys.lock().await.insert(pty_id, PtyHandle { child });

        Response::ok(
            id,
            serde_json::json!({ "pty_id": pty_id, "cols": cols, "rows": rows }),
        )
    }

    async fn pty_write(&self, id: u64, params: &serde_json::Value) -> Response {
        let Some(pty_id) = params.get("pty_id").and_then(|v| v.as_u64()) else {
            return Response::err(id, -32602, "missing `pty_id` param");
        };
        let Some(data_b64) = params.get("data").and_then(|v| v.as_str()) else {
            return Response::err(id, -32602, "missing `data` param");
        };
        let Ok(data) = base64_decode(data_b64) else {
            return Response::err(id, -32602, "invalid base64 data");
        };

        let mut ptys = self.ptys.lock().await;
        let Some(handle) = ptys.get_mut(&pty_id) else {
            return Response::err(id, 1, "pty not found");
        };

        if let Some(ref mut stdin) = handle.child.stdin {
            match stdin.write_all(&data).await {
                Ok(()) => Response::ok(id, serde_json::json!({})),
                Err(e) => Response::err(id, 1, e.to_string()),
            }
        } else {
            Response::err(id, 1, "pty stdin unavailable")
        }
    }

    async fn pty_resize(&self, id: u64, params: &serde_json::Value) -> Response {
        let Some(_pty_id) = params.get("pty_id").and_then(|v| v.as_u64()) else {
            return Response::err(id, -32602, "missing `pty_id` param");
        };
        // Real resize requires OS-level ioctl on the PTY fd; this is a
        // placeholder that acknowledges the request.
        Response::ok(id, serde_json::json!({}))
    }
}

// ---------------------------------------------------------------------------
// Minimal base64 helpers
// ---------------------------------------------------------------------------

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_decode(input: &str) -> Result<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let input = input.trim().as_bytes();
    let mut out = Vec::with_capacity(input.len() * 3 / 4);

    for chunk in input.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let a = val(chunk[0]).unwrap_or(0) as u32;
        let b = val(chunk[1]).unwrap_or(0) as u32;
        let c = if chunk.len() > 2 && chunk[2] != b'=' {
            val(chunk[2]).unwrap_or(0) as u32
        } else {
            0
        };
        let d = if chunk.len() > 3 && chunk[3] != b'=' {
            val(chunk[3]).unwrap_or(0) as u32
        } else {
            0
        };

        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((triple >> 16) as u8);
        if chunk.len() > 2 && chunk[2] != b'=' {
            out.push((triple >> 8) as u8);
        }
        if chunk.len() > 3 && chunk[3] != b'=' {
            out.push(triple as u8);
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        let original = b"Hello, SideX remote server!";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(original.as_slice(), decoded.as_slice());
    }

    #[test]
    fn base64_empty() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_decode("").unwrap(), Vec::<u8>::new());
    }

    #[tokio::test]
    async fn server_handles_unknown_method() {
        let server = SideXServer::new();
        let req = Request {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "unknown/method".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = server.handle(req).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn server_exec_run() {
        let server = SideXServer::new();
        let req = Request {
            jsonrpc: "2.0".to_string(),
            id: 2,
            method: "exec/run".to_string(),
            params: serde_json::json!({ "command": "echo hello" }),
        };
        let resp = server.handle(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert!(result["stdout"].as_str().unwrap().contains("hello"));
    }
}
