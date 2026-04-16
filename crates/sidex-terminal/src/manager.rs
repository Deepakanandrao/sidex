//! Terminal instance manager.
//!
//! Manages multiple terminal instances, each combining a PTY process
//! with a terminal emulator. Provides creation, lookup, resize, output
//! streaming, and removal with proper process-tree cleanup.

use crate::emulator::TerminalEmulator;
use crate::grid::TerminalGrid;
use crate::pty::{PtyError, PtyProcess, PtySpawnConfig, ReadResult, TermHandle, TermInfo, TerminalSize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Unique identifier for a terminal instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerminalId(pub u32);

impl From<TermHandle> for TerminalId {
    fn from(h: TermHandle) -> Self {
        Self(h.0)
    }
}

impl From<TerminalId> for TermHandle {
    fn from(id: TerminalId) -> Self {
        Self(id.0)
    }
}

/// Errors from the terminal manager.
#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("terminal not found: {0:?}")]
    NotFound(TerminalId),
    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),
    #[error("lock poisoned")]
    LockPoisoned,
}

type ManagerResult<T> = Result<T, ManagerError>;

/// Events emitted by terminal instances.
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    /// New data available from the terminal.
    Data { id: TerminalId, text: String },
    /// Terminal process exited.
    Exit { id: TerminalId, exit_code: i32 },
    /// Terminal started.
    Started {
        id: TerminalId,
        shell: String,
        pid: u32,
        cwd: String,
    },
}

/// A single terminal instance: PTY process + terminal emulator.
pub struct TerminalInstance {
    pub pty: PtyProcess,
    pub emulator: TerminalEmulator,
    pub shell: String,
    pub cwd: PathBuf,
    pub size: TerminalSize,
    handle: TermHandle,
}

impl TerminalInstance {
    /// Returns the handle for this instance.
    pub fn handle(&self) -> TermHandle {
        self.handle
    }

    /// Returns a `TermInfo` snapshot.
    pub fn info(&self) -> TermInfo {
        self.pty.info(self.handle)
    }
}

/// Manages a collection of terminal instances.
pub struct TerminalManager {
    terminals: HashMap<TerminalId, Arc<Mutex<TerminalInstance>>>,
    default_size: TerminalSize,
    event_tx: Option<crossbeam::channel::Sender<TerminalEvent>>,
}

impl TerminalManager {
    /// Creates a new, empty terminal manager.
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
            default_size: TerminalSize::default(),
            event_tx: None,
        }
    }

    /// Creates a manager with a custom default terminal size.
    pub fn with_default_size(size: TerminalSize) -> Self {
        Self {
            terminals: HashMap::new(),
            default_size: size,
            event_tx: None,
        }
    }

    /// Sets up an event channel. Returns the receiver.
    pub fn set_event_channel(
        &mut self,
    ) -> crossbeam::channel::Receiver<TerminalEvent> {
        let (tx, rx) = crossbeam::channel::unbounded();
        self.event_tx = Some(tx);
        rx
    }

    /// Creates a new terminal instance with full config and returns its ID.
    pub fn create_with_config(
        &mut self,
        config: &PtySpawnConfig,
    ) -> ManagerResult<TerminalId> {
        let size = config.size;
        let mut pty = PtyProcess::spawn(config)?;

        let grid = TerminalGrid::new(size.rows, size.cols);
        let emulator = TerminalEmulator::new(grid);

        let handle = TermHandle::next();
        let id = TerminalId::from(handle);

        let shell = pty.shell().to_string();
        let cwd = pty.cwd().to_path_buf();
        let pid = pty.pid().unwrap_or(0);

        // Wire up output -> emulator feeding if no event channel is set,
        // or use the event channel for external consumers.
        if let Some(ref tx) = self.event_tx {
            let tx_output = tx.clone();
            let tx_start = tx.clone();
            let term_id = id;
            pty.on_output(move |data| {
                let text = String::from_utf8_lossy(data).to_string();
                let _ = tx_output.send(TerminalEvent::Data {
                    id: term_id,
                    text,
                });
            })?;

            let _ = tx_start.send(TerminalEvent::Started {
                id,
                shell: shell.clone(),
                pid,
                cwd: cwd.to_string_lossy().to_string(),
            });
        }

        let instance = TerminalInstance {
            pty,
            emulator,
            shell,
            cwd,
            size,
            handle,
        };

        self.terminals.insert(id, Arc::new(Mutex::new(instance)));
        Ok(id)
    }

    /// Creates a new terminal instance and returns its ID.
    pub fn create(
        &mut self,
        shell: Option<&str>,
        cwd: Option<&Path>,
    ) -> ManagerResult<TerminalId> {
        self.create_with_size(shell, cwd, self.default_size)
    }

    /// Creates a new terminal with a specific size.
    pub fn create_with_size(
        &mut self,
        shell: Option<&str>,
        cwd: Option<&Path>,
        size: TerminalSize,
    ) -> ManagerResult<TerminalId> {
        let config = PtySpawnConfig {
            shell: shell.map(String::from),
            args: None,
            cwd: cwd.map(Path::to_path_buf),
            env: HashMap::new(),
            size,
        };
        self.create_with_config(&config)
    }

    /// Returns a shared handle to a terminal instance.
    pub fn get(&self, id: TerminalId) -> Option<Arc<Mutex<TerminalInstance>>> {
        self.terminals.get(&id).cloned()
    }

    /// Removes and kills a terminal instance, including its process tree.
    pub fn remove(&mut self, id: TerminalId) -> ManagerResult<()> {
        let instance = self
            .terminals
            .remove(&id)
            .ok_or(ManagerError::NotFound(id))?;
        if let Ok(inst) = instance.lock() {
            let exit_code = inst.pty.exit_code().unwrap_or(0);
            let _ = inst.pty.kill_tree();
            if let Some(ref tx) = self.event_tx {
                let _ = tx.send(TerminalEvent::Exit { id, exit_code });
            }
        }
        Ok(())
    }

    /// Lists all active terminal IDs.
    pub fn list(&self) -> Vec<TerminalId> {
        self.terminals.keys().copied().collect()
    }

    /// Returns the number of active terminals.
    pub fn count(&self) -> usize {
        self.terminals.len()
    }

    /// Reads output from a specific terminal (poll-based).
    pub fn read_output(
        &self,
        id: TerminalId,
        max_lines: Option<usize>,
    ) -> ManagerResult<ReadResult> {
        let instance = self
            .terminals
            .get(&id)
            .ok_or(ManagerError::NotFound(id))?;
        let inst = instance.lock().map_err(|_| ManagerError::LockPoisoned)?;
        inst.pty.read_output(max_lines).map_err(ManagerError::Pty)
    }

    /// Writes input to a specific terminal.
    pub fn write(&self, id: TerminalId, data: &str) -> ManagerResult<()> {
        let instance = self
            .terminals
            .get(&id)
            .ok_or(ManagerError::NotFound(id))?;
        let inst = instance.lock().map_err(|_| ManagerError::LockPoisoned)?;
        inst.pty.write_str(data).map_err(ManagerError::Pty)
    }

    /// Resizes a specific terminal.
    pub fn resize(&self, id: TerminalId, size: TerminalSize) -> ManagerResult<()> {
        let instance = self
            .terminals
            .get(&id)
            .ok_or(ManagerError::NotFound(id))?;
        let mut inst = instance.lock().map_err(|_| ManagerError::LockPoisoned)?;
        inst.pty.resize(size)?;
        inst.emulator.grid_mut().resize(size.rows, size.cols);
        inst.size = size;
        Ok(())
    }

    /// Returns info about a specific terminal.
    pub fn info(&self, id: TerminalId) -> ManagerResult<TermInfo> {
        let instance = self
            .terminals
            .get(&id)
            .ok_or(ManagerError::NotFound(id))?;
        let inst = instance.lock().map_err(|_| ManagerError::LockPoisoned)?;
        Ok(inst.info())
    }

    /// Sends a signal to a terminal's process (Unix only).
    pub fn send_signal(&self, id: TerminalId, signal: i32) -> ManagerResult<()> {
        let instance = self
            .terminals
            .get(&id)
            .ok_or(ManagerError::NotFound(id))?;
        let inst = instance.lock().map_err(|_| ManagerError::LockPoisoned)?;
        if let Some(pid) = inst.pty.pid() {
            crate::pty::send_signal(pid, signal).map_err(ManagerError::Pty)?;
        }
        Ok(())
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}
