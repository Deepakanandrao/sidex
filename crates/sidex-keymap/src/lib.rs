//! Keybinding system for `SideX`.
//!
//! Provides a VS Code-compatible keybinding system with support for key
//! chords, modifier keys, context-aware "when" clauses, and platform-aware
//! default bindings.

pub mod context;
pub mod defaults;
pub mod keybinding;
pub mod resolver;

pub use context::{evaluate, ContextKeys, ContextValue};
pub use defaults::default_keybindings;
pub use keybinding::{Key, KeyBinding, KeyChord, KeyCombo, Modifiers};
pub use resolver::KeybindingResolver;
