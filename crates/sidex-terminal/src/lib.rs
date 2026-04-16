//! Integrated terminal for `SideX` — PTY management and terminal emulation.
//!
//! This crate provides:
//!
//! - **PTY process management** ([`pty`]) — spawn shells, send input, resize,
//!   read output via ring buffer, kill process trees.
//! - **Shell detection** ([`shell`]) — platform-specific default shell,
//!   available shells, and shell integration (zdotdir).
//! - **Terminal grid** ([`grid`]) — character grid with scrollback buffer.
//! - **ANSI emulator** ([`emulator`]) — VTE-based escape sequence parser that
//!   drives the grid.
//! - **Instance manager** ([`manager`]) — manage multiple terminal sessions
//!   with event channels.
//! - **Command execution** ([`exec`]) — non-interactive command execution with
//!   timeout support.

pub mod emulator;
pub mod exec;
pub mod grid;
pub mod manager;
pub mod pty;
pub mod shell;

pub use emulator::TerminalEmulator;
pub use exec::{ExecResult, exec};
pub use grid::{Cell, Color, TerminalGrid};
pub use manager::{ManagerError, TerminalEvent, TerminalId, TerminalInstance, TerminalManager};
pub use pty::{
    OutputChunk, PtyError, PtyProcess, PtySpawnConfig, ReadResult, TermHandle, TermInfo,
    TerminalSize, kill_process_tree, send_signal,
};
pub use shell::{ShellInfo, available_shells, best_shell, check_shell_exists, detect_default_shell, setup_zsh_dotdir};
