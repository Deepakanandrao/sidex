//! Settings/configuration management for `SideX`.
//!
//! Provides a layered settings store (default < user < workspace), JSONC
//! parsing, a schema registry for extensions, and built-in default values
//! matching VS Code conventions.

pub mod defaults;
pub mod jsonc;
pub mod schema;
pub mod settings;

pub use defaults::builtin_defaults;
pub use jsonc::parse_jsonc;
pub use schema::{SchemaRegistry, SettingSchema, SettingType};
pub use settings::Settings;
