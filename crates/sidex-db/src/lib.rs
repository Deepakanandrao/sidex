//! # sidex-db
//!
//! SQLite state persistence for the `SideX` editor.
//!
//! This crate provides durable storage for application state using an
//! embedded SQLite database.  It includes:
//!
//! - [`Database`] — connection wrapper with automatic schema migrations.
//! - [`StateStore`] — scoped key-value store (`global`, `workspace:<path>`,
//!   `extension:<id>`).
//! - [`recent`] — recently opened files and workspaces.
//! - [`window_state`] — window position/layout persistence.

pub mod db;
pub mod recent;
pub mod state;
pub mod window_state;

pub use db::Database;
pub use recent::{
    RecentEntry, add_recent_file, add_recent_workspace, clear_recent, recent_files,
    recent_workspaces,
};
pub use state::StateStore;
pub use window_state::{WindowState, load_window_state, save_window_state};
