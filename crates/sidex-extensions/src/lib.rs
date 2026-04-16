//! Extension host management for `SideX` — Node.js extension host + WASM
//! extensions.
//!
//! This crate provides:
//!
//! - **Extension host** ([`host`]) — spawn and communicate with the Node.js
//!   extension host process via JSON-RPC.
//! - **Manifest parsing** ([`manifest`]) — parse VS Code `package.json` and
//!   `SideX` WASM `sidex.toml` manifests, build init-data payloads.
//! - **Registry** ([`registry`]) — discover, scan, and manage installed
//!   extensions across multiple directories.
//! - **Installer** ([`installer`]) — install, uninstall, and update extensions
//!   from `.vsix` files or the marketplace.
//! - **Marketplace client** ([`marketplace`]) — query the Open VSX registry.
//! - **Paths** ([`paths`]) — standard filesystem paths for extension storage
//!   and Node.js runtime resolution.

pub mod host;
pub mod installer;
pub mod manifest;
pub mod marketplace;
pub mod paths;
pub mod registry;

pub use host::ExtensionHost;
pub use installer::{install_from_marketplace, install_from_vsix, uninstall, update};
pub use manifest::{
    ExtensionContributes, ExtensionDescription, ExtensionHostInitData, ExtensionIdentifier,
    ExtensionKind, ExtensionManifest, UriComponents, build_extension_descriptions,
    build_init_data, is_version_greater, parse_manifest, path_to_uri_path, read_node_manifest,
    read_wasm_manifest, sanitize_ext_id,
};
pub use marketplace::{MarketplaceClient, MarketplaceExtension};
pub use paths::{
    NodeRuntime, global_storage_dir, resolve_node_runtime, sidex_data_dir, user_data_dir,
    user_extensions_dir,
};
pub use registry::{ExtensionRegistry, VsixManifest, read_vsix_manifest};
