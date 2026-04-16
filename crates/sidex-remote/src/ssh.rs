//! SSH remote transport backend.
//!
//! Uses the `russh` crate for async SSH connections, implementing
//! [`RemoteTransport`] with exec, SFTP file operations, PTY channels,
//! and TCP port forwarding.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::{Context, Result, bail};
use russh::client;
use russh_keys::key;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::transport::{DirEntry, ExecOutput, FileStat, RemotePty, RemoteTransport};

// ---------------------------------------------------------------------------
// Auth & config types
// ---------------------------------------------------------------------------

/// Authentication method for an SSH connection.
#[derive(Debug, Clone)]
pub enum SshAuth {
    /// Authenticate with a plaintext password.
    Password(String),
    /// Authenticate with a private key on disk.
    KeyFile(PathBuf),
    /// Authenticate via the running SSH agent.
    Agent,
}

/// Parsed entry from `~/.ssh/config`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshHostConfig {
    pub host_pattern: String,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub identity_file: Option<PathBuf>,
    pub proxy_jump: Option<String>,
}

/// High-level SSH configuration.
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub hosts: Vec<SshHostConfig>,
}

// ---------------------------------------------------------------------------
// SSH config parsing
// ---------------------------------------------------------------------------

/// Parse an OpenSSH-style config file into a list of per-host blocks.
///
/// This handles the most common directives (`Host`, `HostName`, `Port`,
/// `User`, `IdentityFile`, `ProxyJump`).  Unknown directives are silently
/// ignored.
pub fn parse_ssh_config(path: &Path) -> Result<Vec<SshHostConfig>> {
    let contents =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

    let mut hosts = Vec::new();
    let mut current: Option<SshHostConfig> = None;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (keyword, value) = match line.split_once(char::is_whitespace) {
            Some((k, v)) => (k, v.trim()),
            None => continue,
        };

        match keyword.to_lowercase().as_str() {
            "host" => {
                if let Some(entry) = current.take() {
                    hosts.push(entry);
                }
                current = Some(SshHostConfig {
                    host_pattern: value.to_string(),
                    hostname: None,
                    port: None,
                    user: None,
                    identity_file: None,
                    proxy_jump: None,
                });
            }
            "hostname" => {
                if let Some(ref mut entry) = current {
                    entry.hostname = Some(value.to_string());
                }
            }
            "port" => {
                if let Some(ref mut entry) = current {
                    entry.port = value.parse().ok();
                }
            }
            "user" => {
                if let Some(ref mut entry) = current {
                    entry.user = Some(value.to_string());
                }
            }
            "identityfile" => {
                if let Some(ref mut entry) = current {
                    let expanded = if value.starts_with("~/") {
                        dirs::home_dir()
                            .map(|h| h.join(&value[2..]))
                            .unwrap_or_else(|| PathBuf::from(value))
                    } else {
                        PathBuf::from(value)
                    };
                    entry.identity_file = Some(expanded);
                }
            }
            "proxyjump" => {
                if let Some(ref mut entry) = current {
                    entry.proxy_jump = Some(value.to_string());
                }
            }
            _ => {}
        }
    }

    if let Some(entry) = current {
        hosts.push(entry);
    }

    Ok(hosts)
}

// ---------------------------------------------------------------------------
// Client handler (required by russh)
// ---------------------------------------------------------------------------

struct ClientHandler;

#[async_trait::async_trait]
impl client::Handler for ClientHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: implement known-hosts verification
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// SshTransport
// ---------------------------------------------------------------------------

/// SSH-based [`RemoteTransport`] implementation.
pub struct SshTransport {
    session: Arc<Mutex<client::Handle<ClientHandler>>>,
    host: String,
    port: u16,
}

impl SshTransport {
    /// Open an SSH connection to `host:port` using the given authentication.
    pub async fn connect(host: &str, port: u16, auth: SshAuth) -> Result<Self> {
        let config = client::Config::default();
        let config = Arc::new(config);
        let handler = ClientHandler;

        let mut session =
            client::connect(config, (host, port), handler)
                .await
                .with_context(|| format!("SSH connect to {host}:{port}"))?;

        let auth_result = match auth {
            SshAuth::Password(ref password) => {
                session
                    .authenticate_password("root", password)
                    .await
                    .context("SSH password auth")?
            }
            SshAuth::KeyFile(ref key_path) => {
                let key_pair = russh_keys::load_secret_key(key_path, None)
                    .with_context(|| format!("loading SSH key {}", key_path.display()))?;
                session
                    .authenticate_publickey("root", Arc::new(key_pair))
                    .await
                    .context("SSH pubkey auth")?
            }
            SshAuth::Agent => {
                let default_key = dirs::home_dir()
                    .map(|h| h.join(".ssh/id_ed25519"))
                    .or_else(|| dirs::home_dir().map(|h| h.join(".ssh/id_rsa")));
                let Some(key_path) = default_key.filter(|p| p.exists()) else {
                    bail!("SSH agent auth: no default key found in ~/.ssh/");
                };
                let key_pair = russh_keys::load_secret_key(&key_path, None)
                    .with_context(|| format!("loading SSH key {}", key_path.display()))?;
                session
                    .authenticate_publickey("root", Arc::new(key_pair))
                    .await
                    .context("SSH agent auth")?
            }
        };

        if !auth_result {
            bail!("SSH authentication failed for {host}:{port}");
        }

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            host: host.to_string(),
            port,
        })
    }

    /// Forward a local TCP port to a remote address through the SSH tunnel.
    pub async fn forward_port(
        &self,
        local_port: u16,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<()> {
        let session = Arc::clone(&self.session);
        let remote_host = remote_host.to_string();
        let log_host = remote_host.clone();

        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], local_port)))
            .await
            .with_context(|| format!("binding local port {local_port}"))?;

        tokio::spawn(async move {
            loop {
                let Ok((mut local_stream, _)) = listener.accept().await else {
                    break;
                };

                let session = Arc::clone(&session);
                let rh = remote_host.clone();

                tokio::spawn(async move {
                    let channel = {
                        let sess = session.lock().await;
                        match sess
                            .channel_open_direct_tcpip(&rh, remote_port.into(), "127.0.0.1", 0)
                            .await
                        {
                            Ok(ch) => ch,
                            Err(e) => {
                                log::error!("port-forward channel open: {e}");
                                return;
                            }
                        }
                    };

                    let mut remote_stream = channel.into_stream();
                    if let Err(e) =
                        tokio::io::copy_bidirectional(&mut local_stream, &mut remote_stream).await
                    {
                        log::debug!("port-forward stream ended: {e}");
                    }
                });
            }
        });

        log::info!(
            "forwarding 127.0.0.1:{local_port} -> {log_host}:{remote_port} via {}:{}",
            self.host,
            self.port
        );

        Ok(())
    }

    /// Run a command over SSH and collect output.
    async fn exec_inner(&self, command: &str) -> Result<ExecOutput> {
        let session = self.session.lock().await;
        let channel = session.channel_open_session().await?;
        channel.exec(true, command).await?;
        drop(session);

        let mut stdout = Vec::new();
        let stderr = Vec::new();
        let exit_code: i32 = -1;

        let mut stream = channel.into_stream();
        let mut buf = [0u8; 8192];
        loop {
            match stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => stdout.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }

        Ok(ExecOutput {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            exit_code,
        })
    }
}

#[async_trait::async_trait]
impl RemoteTransport for SshTransport {
    async fn exec(&self, command: &str) -> Result<ExecOutput> {
        self.exec_inner(command).await
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let out = self.exec_inner(&format!("cat {path:?}")).await?;
        if out.exit_code != 0 {
            bail!("read_file({path}): {}", out.stderr);
        }
        Ok(out.stdout.into_bytes())
    }

    async fn write_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let encoded = base64_encode(data);
        let cmd = format!("echo '{encoded}' | base64 -d > {path:?}");
        let out = self.exec_inner(&cmd).await?;
        if out.exit_code != 0 {
            bail!("write_file({path}): {}", out.stderr);
        }
        Ok(())
    }

    async fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>> {
        let cmd = format!(
            "find {path:?} -maxdepth 1 -mindepth 1 \
             -printf '%f\\t%s\\t%y\\t%T@\\t%p\\n' 2>/dev/null || \
             ls -1 {path:?}"
        );
        let out = self.exec_inner(&cmd).await?;
        let mut entries = Vec::new();

        for line in out.stdout.lines() {
            let parts: Vec<&str> = line.splitn(5, '\t').collect();
            if parts.len() == 5 {
                let modified = parts[3]
                    .parse::<f64>()
                    .ok()
                    .and_then(|secs| {
                        SystemTime::UNIX_EPOCH.checked_add(
                            std::time::Duration::from_secs_f64(secs),
                        )
                    });
                entries.push(DirEntry {
                    name: parts[0].to_string(),
                    path: parts[4].to_string(),
                    is_dir: parts[2] == "d",
                    size: parts[1].parse().unwrap_or(0),
                    modified,
                });
            } else {
                entries.push(DirEntry {
                    name: line.to_string(),
                    path: format!("{path}/{line}"),
                    is_dir: false,
                    size: 0,
                    modified: None,
                });
            }
        }

        Ok(entries)
    }

    async fn stat(&self, path: &str) -> Result<FileStat> {
        let cmd = format!("stat -c '%s %Y %F %h' {path:?} 2>/dev/null || stat -f '%z %m %T %l' {path:?}");
        let out = self.exec_inner(&cmd).await?;
        if out.exit_code != 0 {
            bail!("stat({path}): {}", out.stderr);
        }

        let parts: Vec<&str> = out.stdout.trim().splitn(4, ' ').collect();
        if parts.len() < 4 {
            bail!("unexpected stat output for {path}: {}", out.stdout);
        }

        let size = parts[0].parse().unwrap_or(0);
        let modified_secs: u64 = parts[1].parse().unwrap_or(0);
        let modified = SystemTime::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(modified_secs));
        let file_type = parts[2];

        Ok(FileStat {
            size,
            modified,
            is_dir: file_type.contains("directory"),
            is_symlink: file_type.contains("symbolic") || file_type.contains("link"),
        })
    }

    async fn open_pty(&self, cols: u16, rows: u16) -> Result<RemotePty> {
        let session = self.session.lock().await;
        let channel = session.channel_open_session().await?;
        channel
            .request_pty(
                true,
                "xterm-256color",
                cols.into(),
                rows.into(),
                0,
                0,
                &[],
            )
            .await?;
        channel.request_shell(true).await?;
        drop(session);

        let (resize_tx, mut resize_rx) = tokio::sync::mpsc::channel::<(u16, u16)>(8);

        let stream = channel.into_stream();
        let (reader, writer) = tokio::io::split(stream);

        // Resize handling would be done via a separate channel request;
        // for now we consume the resize events (real implementation would
        // send window-change requests on the SSH channel).
        tokio::spawn(async move {
            while let Some((_c, _r)) = resize_rx.recv().await {
                // channel.window_change(c, r, 0, 0) — requires channel handle
            }
        });

        Ok(RemotePty::new(
            Box::new(writer),
            Box::new(reader),
            resize_tx,
        ))
    }

    async fn upload(&self, local: &Path, remote: &str) -> Result<()> {
        let data = tokio::fs::read(local)
            .await
            .with_context(|| format!("reading local file {}", local.display()))?;
        self.write_file(remote, &data).await
    }

    async fn download(&self, remote: &str, local: &Path) -> Result<()> {
        let data = self.read_file(remote).await?;
        tokio::fs::write(local, &data)
            .await
            .with_context(|| format!("writing local file {}", local.display()))?;
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        let session = self.session.lock().await;
        session
            .disconnect(russh::Disconnect::ByApplication, "bye", "en")
            .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
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

// ---------------------------------------------------------------------------
// Resolve host alias via ssh config
// ---------------------------------------------------------------------------

impl SshConfig {
    /// Look up a host alias and return the resolved hostname, port, user,
    /// identity file, and proxy jump (if any).
    pub fn resolve(&self, alias: &str) -> Option<&SshHostConfig> {
        self.hosts.iter().find(|h| {
            let pat = &h.host_pattern;
            if let Some(suffix) = pat.strip_prefix('*') {
                alias.ends_with(suffix)
            } else if let Some(prefix) = pat.strip_suffix('*') {
                alias.starts_with(prefix)
            } else {
                pat == alias
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_config(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parse_simple_ssh_config() {
        let cfg = write_config(
            "\
Host myserver
    HostName 192.168.1.100
    Port 2222
    User deploy
    IdentityFile ~/.ssh/deploy_key

Host *.example.com
    User admin
    ProxyJump bastion
",
        );

        let hosts = parse_ssh_config(cfg.path()).unwrap();
        assert_eq!(hosts.len(), 2);

        assert_eq!(hosts[0].host_pattern, "myserver");
        assert_eq!(hosts[0].hostname.as_deref(), Some("192.168.1.100"));
        assert_eq!(hosts[0].port, Some(2222));
        assert_eq!(hosts[0].user.as_deref(), Some("deploy"));
        assert!(hosts[0].identity_file.is_some());

        assert_eq!(hosts[1].host_pattern, "*.example.com");
        assert_eq!(hosts[1].user.as_deref(), Some("admin"));
        assert_eq!(hosts[1].proxy_jump.as_deref(), Some("bastion"));
    }

    #[test]
    fn parse_empty_config() {
        let cfg = write_config("");
        let hosts = parse_ssh_config(cfg.path()).unwrap();
        assert!(hosts.is_empty());
    }

    #[test]
    fn config_resolve_exact() {
        let config = SshConfig {
            hosts: vec![SshHostConfig {
                host_pattern: "prod".to_string(),
                hostname: Some("10.0.0.1".to_string()),
                port: Some(22),
                user: Some("root".to_string()),
                identity_file: None,
                proxy_jump: None,
            }],
        };

        assert!(config.resolve("prod").is_some());
        assert!(config.resolve("staging").is_none());
    }

    #[test]
    fn config_resolve_wildcard() {
        let config = SshConfig {
            hosts: vec![SshHostConfig {
                host_pattern: "*.dev".to_string(),
                hostname: None,
                port: None,
                user: Some("dev".to_string()),
                identity_file: None,
                proxy_jump: None,
            }],
        };

        assert!(config.resolve("api.dev").is_some());
        assert!(config.resolve("prod.com").is_none());
    }
}
