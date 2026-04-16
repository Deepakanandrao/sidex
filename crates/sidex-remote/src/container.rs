//! Dev Container remote transport backend.
//!
//! Uses the `bollard` crate for Docker API access.  Parses
//! `.devcontainer/devcontainer.json` and manages container lifecycle.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, LogOutput, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::BuildImageOptions;
use bollard::Docker;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::transport::{DirEntry, ExecOutput, FileStat, RemotePty, RemoteTransport};

// ---------------------------------------------------------------------------
// Config types
// ---------------------------------------------------------------------------

/// Parsed representation of `.devcontainer/devcontainer.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DevContainerConfig {
    /// Pre-built image to use (e.g. `mcr.microsoft.com/devcontainers/rust:1`).
    #[serde(default)]
    pub image: Option<String>,

    /// Path to a `Dockerfile` (relative to `.devcontainer/`).
    #[serde(default)]
    pub dockerfile: Option<String>,

    /// Path to a `docker-compose.yml`.
    #[serde(default, rename = "dockerComposeFile")]
    pub docker_compose_file: Option<String>,

    /// Ports to forward from the container to the host.
    #[serde(default)]
    pub forward_ports: Vec<u16>,

    /// Volume mounts.
    #[serde(default)]
    pub mounts: Vec<Mount>,

    /// Command to run after the container is created.
    #[serde(default)]
    pub post_create_command: Option<String>,

    /// Dev Container Features to install.
    #[serde(default)]
    pub features: HashMap<String, Value>,
}

/// A bind mount specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    pub source: String,
    pub target: String,
    #[serde(default = "default_mount_type")]
    pub r#type: String,
}

fn default_mount_type() -> String {
    "bind".to_string()
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a `devcontainer.json` file, stripping JSON-with-comments before
/// deserializing.
pub fn parse_devcontainer(path: &Path) -> Result<DevContainerConfig> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let stripped = strip_jsonc_comments(&raw);
    let config: DevContainerConfig =
        serde_json::from_str(&stripped).context("parsing devcontainer.json")?;
    Ok(config)
}

/// Minimal JSONC comment stripper (single-line `//` and block `/* */`).
fn strip_jsonc_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(&next) = chars.peek() {
                    out.push(next);
                    chars.next();
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        if c == '"' {
            in_string = true;
            out.push(c);
            continue;
        }

        if c == '/' {
            match chars.peek() {
                Some(&'/') => {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                }
                Some(&'*') => {
                    chars.next();
                    loop {
                        match chars.next() {
                            Some('*') if chars.peek() == Some(&'/') => {
                                chars.next();
                                break;
                            }
                            Some('\n') => out.push('\n'),
                            None => break,
                            _ => {}
                        }
                    }
                }
                _ => out.push(c),
            }
        } else {
            out.push(c);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Container transport
// ---------------------------------------------------------------------------

/// Dev-Container-based [`RemoteTransport`].
pub struct ContainerTransport {
    docker: Docker,
    container_id: String,
}

impl ContainerTransport {
    /// Build (if needed) and start a dev container from the given config.
    pub async fn start(config: &DevContainerConfig, workspace_path: &Path) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .context("connecting to Docker daemon")?;

        let image = if let Some(ref img) = config.image {
            img.clone()
        } else if config.dockerfile.is_some() {
            Self::build_image_inner(&docker, config, workspace_path).await?
        } else {
            bail!("devcontainer.json must specify either `image` or `dockerfile`");
        };

        let id = Self::create_container_inner(
            &docker,
            &image,
            workspace_path,
            &config.mounts,
            &config.forward_ports,
        )
        .await?;

        docker
            .start_container(&id, None::<StartContainerOptions<String>>)
            .await
            .context("starting container")?;

        if let Some(ref cmd) = config.post_create_command {
            let transport = Self {
                docker: docker.clone(),
                container_id: id.clone(),
            };
            let out = transport.exec(cmd).await?;
            if out.exit_code != 0 {
                log::warn!("postCreateCommand failed: {}", out.stderr);
            }
        }

        Ok(Self {
            docker,
            container_id: id,
        })
    }

    async fn build_image_inner(
        docker: &Docker,
        config: &DevContainerConfig,
        context_path: &Path,
    ) -> Result<String> {
        let tag = format!("sidex-devcontainer:{:x}", fxhash(context_path));
        let dockerfile = config.dockerfile.as_deref().unwrap_or("Dockerfile");

        let opts = BuildImageOptions {
            dockerfile: dockerfile.to_string(),
            t: tag.clone(),
            ..Default::default()
        };

        use bollard::models::BuildInfo;
        use futures_util::StreamExt;

        let tar_bytes = tar_directory(context_path)?;
        let mut stream =
            docker.build_image(opts, None, Some(tar_bytes.into()));

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(BuildInfo {
                    stream: Some(ref s), ..
                }) => log::debug!("{}", s.trim_end()),
                Ok(BuildInfo {
                    error: Some(ref e), ..
                }) => bail!("docker build error: {e}"),
                Err(e) => bail!("docker build stream error: {e}"),
                _ => {}
            }
        }

        Ok(tag)
    }

    async fn create_container_inner(
        docker: &Docker,
        image: &str,
        workspace_path: &Path,
        mounts: &[Mount],
        forward_ports: &[u16],
    ) -> Result<String> {
        let workspace_str = workspace_path.to_string_lossy();
        let mut binds = vec![format!("{workspace_str}:/workspace")];
        for m in mounts {
            binds.push(format!("{}:{}:{}", m.source, m.target, m.r#type));
        }

        let exposed: HashMap<String, HashMap<(), ()>> = forward_ports
            .iter()
            .map(|p| (format!("{p}/tcp"), HashMap::new()))
            .collect();

        let host_config = bollard::models::HostConfig {
            binds: Some(binds),
            ..Default::default()
        };

        let container_config = ContainerConfig {
            image: Some(image.to_string()),
            cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
            working_dir: Some("/workspace".to_string()),
            host_config: Some(host_config),
            exposed_ports: Some(exposed),
            ..Default::default()
        };

        let name = format!("sidex-{:x}", fxhash(workspace_path));
        let opts = CreateContainerOptions {
            name: name.clone(),
            platform: None,
        };

        let resp = docker
            .create_container(Some(opts), container_config)
            .await
            .context("creating container")?;

        Ok(resp.id)
    }

    /// Build a Docker image from the dev container config.
    pub async fn build_image(
        config: &DevContainerConfig,
        workspace_path: &Path,
    ) -> Result<String> {
        let docker = Docker::connect_with_local_defaults()?;
        Self::build_image_inner(&docker, config, workspace_path).await
    }

    /// Stop a running container.
    pub async fn stop_container(id: &str) -> Result<()> {
        let docker = Docker::connect_with_local_defaults()?;
        docker
            .stop_container(id, Some(StopContainerOptions { t: 10 }))
            .await?;
        Ok(())
    }

    /// Remove a container.
    pub async fn remove_container(id: &str) -> Result<()> {
        let docker = Docker::connect_with_local_defaults()?;
        docker
            .remove_container(
                id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl RemoteTransport for ContainerTransport {
    async fn exec(&self, command: &str) -> Result<ExecOutput> {
        let exec = self
            .docker
            .create_exec(
                &self.container_id,
                CreateExecOptions {
                    cmd: Some(vec!["sh", "-c", command]),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    ..Default::default()
                },
            )
            .await?;

        let output = self.docker.start_exec(&exec.id, None).await?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } = output {
            use futures_util::StreamExt;
            while let Some(Ok(msg)) = output.next().await {
                match msg {
                    LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }

        let inspect = self.docker.inspect_exec(&exec.id).await?;
        let exit_code = inspect.exit_code.unwrap_or(-1);

        Ok(ExecOutput {
            stdout,
            stderr,
            exit_code: exit_code as i32,
        })
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let out = self.exec(&format!("cat {path:?}")).await?;
        if out.exit_code != 0 {
            bail!("read_file({path}): {}", out.stderr);
        }
        Ok(out.stdout.into_bytes())
    }

    async fn write_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let encoded = base64_encode_simple(data);
        let cmd = format!("echo '{encoded}' | base64 -d > {path:?}");
        let out = self.exec(&cmd).await?;
        if out.exit_code != 0 {
            bail!("write_file({path}): {}", out.stderr);
        }
        Ok(())
    }

    async fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>> {
        let cmd = format!(
            "find {path:?} -maxdepth 1 -mindepth 1 \
             -printf '%f\\t%s\\t%y\\t%T@\\t%p\\n'"
        );
        let out = self.exec(&cmd).await?;
        let mut entries = Vec::new();
        for line in out.stdout.lines() {
            let parts: Vec<&str> = line.splitn(5, '\t').collect();
            if parts.len() == 5 {
                let modified = parts[3].parse::<f64>().ok().and_then(|secs| {
                    std::time::SystemTime::UNIX_EPOCH
                        .checked_add(std::time::Duration::from_secs_f64(secs))
                });
                entries.push(DirEntry {
                    name: parts[0].to_string(),
                    path: parts[4].to_string(),
                    is_dir: parts[2] == "d",
                    size: parts[1].parse().unwrap_or(0),
                    modified,
                });
            }
        }
        Ok(entries)
    }

    async fn stat(&self, path: &str) -> Result<FileStat> {
        let cmd = format!("stat -c '%s %Y %F' {path:?}");
        let out = self.exec(&cmd).await?;
        if out.exit_code != 0 {
            bail!("stat({path}): {}", out.stderr);
        }
        let parts: Vec<&str> = out.stdout.trim().splitn(3, ' ').collect();
        if parts.len() < 3 {
            bail!("unexpected stat output: {}", out.stdout);
        }
        let size = parts[0].parse().unwrap_or(0);
        let modified = parts[1].parse::<u64>().ok().and_then(|s| {
            std::time::SystemTime::UNIX_EPOCH
                .checked_add(std::time::Duration::from_secs(s))
        });
        Ok(FileStat {
            size,
            modified,
            is_dir: parts[2].contains("directory"),
            is_symlink: parts[2].contains("symbolic"),
        })
    }

    async fn open_pty(&self, _cols: u16, _rows: u16) -> Result<RemotePty> {
        bail!("container PTY: use docker exec -it (not yet wired to RemotePty)")
    }

    async fn upload(&self, local: &Path, remote: &str) -> Result<()> {
        let data = tokio::fs::read(local).await?;
        self.write_file(remote, &data).await
    }

    async fn download(&self, remote: &str, local: &Path) -> Result<()> {
        let data = self.read_file(remote).await?;
        tokio::fs::write(local, &data).await?;
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.docker
            .stop_container(&self.container_id, Some(StopContainerOptions { t: 10 }))
            .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Minimal base64 encoder (no external dep).
fn base64_encode_simple(data: &[u8]) -> String {
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

/// Quick non-cryptographic hash for generating deterministic container names.
fn fxhash(path: &Path) -> u64 {
    let s = path.to_string_lossy();
    let mut hash: u64 = 0;
    for b in s.bytes() {
        hash = hash.wrapping_mul(0x0100_0000_01b3).wrapping_add(u64::from(b));
    }
    hash
}

/// Create a tar archive of a directory in memory (for docker build context).
fn tar_directory(dir: &Path) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut ar = tar::Builder::new(&mut buf);
        ar.append_dir_all(".", dir)
            .with_context(|| format!("archiving {}", dir.display()))?;
        ar.finish()?;
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_devcontainer() {
        let json = r#"{ "image": "mcr.microsoft.com/devcontainers/rust:1" }"#;
        let tmp = std::env::temp_dir().join("test_devcontainer.json");
        std::fs::write(&tmp, json).unwrap();
        let config = parse_devcontainer(&tmp).unwrap();
        assert_eq!(
            config.image.as_deref(),
            Some("mcr.microsoft.com/devcontainers/rust:1")
        );
        assert!(config.features.is_empty());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn parse_devcontainer_with_comments() {
        let jsonc = r#"{
  // this is a comment
  "image": "node:20",
  "forwardPorts": [3000, 8080],
  /* block comment */
  "postCreateCommand": "npm install",
  "features": {
    "ghcr.io/devcontainers/features/git:1": {}
  }
}"#;
        let tmp = std::env::temp_dir().join("test_devcontainer_comments.json");
        std::fs::write(&tmp, jsonc).unwrap();
        let config = parse_devcontainer(&tmp).unwrap();
        assert_eq!(config.image.as_deref(), Some("node:20"));
        assert_eq!(config.forward_ports, vec![3000, 8080]);
        assert_eq!(config.post_create_command.as_deref(), Some("npm install"));
        assert_eq!(config.features.len(), 1);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn strip_jsonc_preserves_strings() {
        let input = r#"{"url": "https://example.com/path"}"#;
        let out = strip_jsonc_comments(input);
        assert_eq!(input, out);
    }
}
