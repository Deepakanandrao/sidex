//! Node.js extension host process management.
//!
//! Spawns the VS Code-compatible Node.js extension host as a child process and
//! communicates via JSON-RPC over stdin/stdout. Supports sending requests and
//! notifications in both directions.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Notify, mpsc, oneshot};

/// A pending request awaiting its response.
type PendingRequest = oneshot::Sender<Result<Value>>;

/// JSON-RPC message used on the wire between the editor and the extension host.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JsonRpcMessage {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

/// Callback invoked when the extension host sends a request to the editor.
pub type RequestHandler = Arc<dyn Fn(&str, Value) -> Result<Value> + Send + Sync>;

/// Callback invoked when the extension host sends a notification to the editor.
pub type NotificationHandler = Arc<dyn Fn(&str, Value) + Send + Sync>;

/// Manages the lifecycle of a Node.js extension host child process.
///
/// Communicates using a simplified JSON-RPC protocol over stdin/stdout,
/// allowing the editor to invoke extension-host APIs and vice versa.
pub struct ExtensionHost {
    child: Child,
    next_id: Arc<AtomicU64>,
    pending: Arc<Mutex<HashMap<u64, PendingRequest>>>,
    writer_tx: mpsc::Sender<String>,
    shutdown_signal: Arc<Notify>,
}

impl ExtensionHost {
    /// Spawns the Node.js extension host process.
    ///
    /// * `node_path` — path to the `node` binary.
    /// * `host_script` — path to the JS entry point for the extension host.
    /// * `extensions_dir` — directory containing installed extensions.
    pub fn start(
        node_path: &str,
        host_script: &Path,
        extensions_dir: &Path,
    ) -> Result<Self> {
        let mut child = Command::new(node_path)
            .arg(host_script)
            .arg("--extensions-dir")
            .arg(extensions_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("failed to spawn Node.js extension host")?;

        let stdout = child.stdout.take().context("missing stdout")?;
        let stdin = child.stdin.take().context("missing stdin")?;

        let pending: Arc<Mutex<HashMap<u64, PendingRequest>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let next_id = Arc::new(AtomicU64::new(1));
        let shutdown_signal = Arc::new(Notify::new());

        let (writer_tx, mut writer_rx) = mpsc::channel::<String>(256);

        // Writer task — serialises outbound messages to stdin.
        let shutdown_w = shutdown_signal.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            loop {
                tokio::select! {
                    msg = writer_rx.recv() => {
                        match msg {
                            Some(line) => {
                                if stdin.write_all(line.as_bytes()).await.is_err() {
                                    break;
                                }
                                let _ = stdin.flush().await;
                            }
                            None => break,
                        }
                    }
                    () = shutdown_w.notified() => break,
                }
            }
        });

        // Reader task — reads JSON-RPC responses from stdout and resolves
        // pending futures.
        let pending_r = pending.clone();
        let shutdown_r = shutdown_signal.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            loop {
                tokio::select! {
                    line = lines.next_line() => {
                        match line {
                            Ok(Some(text)) => {
                                Self::handle_incoming(&text, &pending_r).await;
                            }
                            Ok(None) | Err(_) => break,
                        }
                    }
                    () = shutdown_r.notified() => break,
                }
            }
        });

        Ok(Self {
            child,
            next_id,
            pending,
            writer_tx,
            shutdown_signal,
        })
    }

    /// Sends a JSON-RPC request and waits for the response.
    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let msg = JsonRpcMessage {
            jsonrpc: "2.0".to_owned(),
            id: Some(id),
            method: Some(method.to_owned()),
            params: Some(params),
            result: None,
            error: None,
        };

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let line = serde_json::to_string(&msg)? + "\n";
        self.writer_tx
            .send(line)
            .await
            .map_err(|_| anyhow::anyhow!("host writer channel closed"))?;

        rx.await.map_err(|_| anyhow::anyhow!("response channel dropped"))?
    }

    /// Sends a fire-and-forget notification to the extension host.
    pub async fn send_notification(&self, method: &str, params: Value) -> Result<()> {
        let msg = JsonRpcMessage {
            jsonrpc: "2.0".to_owned(),
            id: None,
            method: Some(method.to_owned()),
            params: Some(params),
            result: None,
            error: None,
        };
        let line = serde_json::to_string(&msg)? + "\n";
        self.writer_tx
            .send(line)
            .await
            .map_err(|_| anyhow::anyhow!("host writer channel closed"))?;
        Ok(())
    }

    /// Gracefully shuts down the extension host process.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.shutdown_signal.notify_waiters();

        let _ = self.send_notification("shutdown", Value::Null).await;

        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.child.wait(),
        )
        .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e).context("error waiting for extension host"),
            Err(_) => {
                self.child.kill().await.context("failed to kill extension host")?;
                Ok(())
            }
        }
    }

    /// Processes an incoming JSON-RPC line from the extension host.
    async fn handle_incoming(
        text: &str,
        pending: &Arc<Mutex<HashMap<u64, PendingRequest>>>,
    ) {
        let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(text) else {
            log::warn!("malformed JSON-RPC from extension host: {text}");
            return;
        };

        // Response to one of our requests.
        if let Some(id) = msg.id {
            if msg.result.is_some() || msg.error.is_some() {
                if let Some(tx) = pending.lock().await.remove(&id) {
                    let result = if let Some(err) = msg.error {
                        Err(anyhow::anyhow!("extension host error: {err}"))
                    } else {
                        Ok(msg.result.unwrap_or(Value::Null))
                    };
                    let _ = tx.send(result);
                }
            }
        }
    }
}
