//! # sidex-app
//!
//! Main application binary for the `SideX` code editor.
//!
//! Re-exports the core [`App`] type and supporting modules for use by
//! integration tests or embedding scenarios.

pub mod app;
pub mod clipboard;
pub mod commands;
pub mod document_state;
pub mod event_loop;
pub mod file_dialog;
pub mod layout;

pub use app::App;
pub use commands::{CommandRegistry, NavigationEntry};
pub use document_state::DocumentState;
pub use layout::{Layout, LayoutRects, Rect};
