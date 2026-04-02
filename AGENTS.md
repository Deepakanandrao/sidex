# SideX - AI Agent Guide

This document provides essential information for AI coding agents working on SideX, a clean-room Tauri port of Visual Studio Code.

## Project Overview

**SideX** is a complete 1:1 architectural replica of VSCode, replacing Electron with Tauri (Rust backend + native webview). The project follows the [Open Claw](https://github.com/instructkr/claw-code) methodology — studying the original architecture, mapping every subsystem, and systematically porting to a new runtime.

- **5,600+ TypeScript files** ported from VSCode's source
- **335 CSS files** for the complete UI
- **36 Rust files** replacing Electron's main process
- **Zero `from 'electron'` imports** remaining in the codebase

## Technology Stack

### Frontend (TypeScript)
- **Build Tool**: Vite 6.x
- **Language**: TypeScript 5.6 (ES2022 target)
- **Module System**: ESNext with `type: "module"`
- **UI Framework**: Custom (VSCode's original DOM-based UI)
- **Editor**: Monaco Editor (from VSCode source)
- **Terminal**: xterm.js with multiple addons

### Backend (Rust)
- **Framework**: Tauri 2.x
- **Rust Edition**: 2021 (minimum version 1.77.2)
- **Key Crates**:
  - `portable-pty` - Terminal PTY support
  - `notify` - File system watching
  - `rusqlite` - SQLite storage
  - `tokio` - Async runtime
  - `reqwest` - HTTP client
  - `walkdir` - Directory traversal

### Architecture Mapping

```
VSCode (Electron)                    SideX (Tauri)
─────────────────                    ─────────────
Electron Main Process (94 files)  →  Tauri Rust Backend (36 files)
  BrowserWindow                   →  WebviewWindow
  ipcMain/ipcRenderer             →  invoke() + listen()/emit()
  Menu/MenuItem                   →  tauri::menu
  dialog.*                        →  @tauri-apps/plugin-dialog
  clipboard                       →  @tauri-apps/plugin-clipboard-manager
  shell.openExternal              →  @tauri-apps/plugin-opener
  Notification                    →  @tauri-apps/plugin-notification
  safeStorage                     →  Rust keyring
  protocol.*                      →  Tauri custom protocol
  screen                          →  Tauri monitor API
  net.fetch                       →  Browser fetch() API
  powerMonitor                    →  Rust sysinfo
  
Renderer Process (244 files)      →  Tauri Webview (direct)
  contextBridge/preload           →  @tauri-apps/api (no bridge needed)
  ipcRenderer                     →  invoke() from @tauri-apps/api/core
  webFrame                        →  CSS zoom

Node.js Layer (262 files)         →  Tauri invoke() → Rust
  fs/fs.promises                  →  invoke('fs_*') → Rust std::fs
  child_process                   →  invoke('process_*') → Rust Command
  node-pty                        →  invoke('terminal_*') → portable-pty
  @parcel/watcher                 →  invoke('fs_watch') → notify crate
  net/http/https                  →  fetch() API / invoke() → reqwest
  crypto                          →  Web Crypto API / invoke() → ring
  os.*                            →  invoke('os_*') → Rust sysinfo
  @vscode/sqlite3                 →  invoke('storage_*') → rusqlite
  @vscode/spdlog                  →  invoke('log_*') → tracing
```

## Project Structure

```
sidex/
├── src/                           # ~6,000+ TypeScript files (VSCode workbench)
│   ├── vs/
│   │   ├── base/                  # Foundation utilities (430 TS files)
│   │   │   ├── common/            # Pure TypeScript utilities
│   │   │   ├── browser/           # DOM utilities
│   │   │   ├── node/              # Node.js → Tauri invoke() bridge
│   │   │   └── parts/             # IPC, storage, sandbox
│   │   ├── platform/              # 93 platform services (745 TS files)
│   │   │   ├── files/             # File system service
│   │   │   ├── windows/           # Window management
│   │   │   ├── terminal/          # Terminal service
│   │   │   ├── configuration/     # Settings
│   │   │   └── ... (93 total)
│   │   ├── editor/                # Monaco editor (852 TS files)
│   │   │   ├── common/            # Editor model, languages
│   │   │   ├── browser/           # Editor widget
│   │   │   ├── contrib/           # 57 editor contributions
│   │   │   └── standalone/        # Standalone editor API
│   │   ├── workbench/             # IDE shell (3,269 TS files)
│   │   │   ├── browser/           # Layout, Parts, boot
│   │   │   ├── common/            # Shared types
│   │   │   ├── contrib/           # 95 feature contributions
│   │   │   ├── services/          # 92 workbench services
│   │   │   └── api/               # Extension host API
│   │   ├── code/                  # Application entry (15 files)
│   │   └── server/                # Server/remote support (23 files)
│   ├── typings/                   # Type declarations
│   ├── vscode-dts/                # VS Code API types
│   ├── main.ts                    # Frontend entry point
│   └── styles.css                 # Theme
├── src-tauri/                     # Rust backend (Tauri)
│   ├── src/
│   │   ├── commands/              # Tauri command handlers
│   │   │   ├── fs.rs              # File system commands
│   │   │   ├── window.rs          # Window management
│   │   │   ├── terminal.rs        # PTY/terminal commands
│   │   │   ├── process.rs         # Process management
│   │   │   ├── os.rs              # OS information
│   │   │   ├── storage.rs         # SQLite storage
│   │   │   ├── search.rs          # File/text search
│   │   │   ├── git.rs             # Git operations
│   │   │   ├── proxy.rs           # HTTP proxy for OpenVSX
│   │   │   ├── ext_host.rs        # Extension host management
│   │   │   ├── debug.rs           # Debug adapter
│   │   │   └── tasks.rs           # Task runner
│   │   ├── lib.rs                 # Tauri app setup, menu, commands
│   │   └── main.rs                # Entry point
│   ├── Cargo.toml                 # Rust dependencies
│   └── tauri.conf.json            # Tauri configuration
├── index.html                     # HTML entry with globals setup
├── vite.config.ts                 # Vite configuration
├── tsconfig.json                  # TypeScript configuration
├── package.json                   # Node.js dependencies
├── port_manifest.json             # Machine-readable port status
├── ARCHITECTURE.md                # Detailed architecture documentation
└── README.md                      # User-facing documentation
```

## Build and Run Commands

### Prerequisites
- Node.js 20+
- Rust 1.77.2+
- Tauri CLI (installed via npm)

### Development

```bash
# Install dependencies
npm install

# Start development server with hot reload
npm run tauri dev

# Start only Vite dev server (frontend only)
npm run dev

# Preview production build
npm run preview
```

### Production Build

```bash
# Build release for current platform
npm run tauri build

# Output locations:
# macOS: src-tauri/target/release/bundle/macos/SideX.app
# DMG:   src-tauri/target/release/bundle/dmg/SideX_*.dmg
# Windows: src-tauri/target/release/bundle/msi/SideX_*.msi
# Linux: src-tauri/target/release/bundle/appimage/SideX_*.AppImage
```

### Type Checking

```bash
# TypeScript type check (no emit)
npx tsc --noEmit

# Rust check
cd src-tauri && cargo check
```

## Key Configuration Files

### Vite Configuration (vite.config.ts)
- **Port**: 1420 (strict)
- **Path Alias**: `vs` → `src/vs`
- **Static Copy**: `extensions/`, `extensions-meta.json`, `onig.wasm`
- **Build Target**: ES2022, Chrome 100, Safari 15
- **Special Handling**: WASM files, manual chunks for Monaco

### TypeScript Configuration (tsconfig.json)
- **Target**: ES2022
- **Module**: ESNext with bundler resolution
- **Path Mapping**: `vs/*` → `src/vs/*`
- **Strict Mode**: OFF (relaxed for compatibility with VSCode source)
- **Decorator Support**: Enabled (experimental)

### Tauri Configuration (src-tauri/Cargo.toml)
- **Tauri Version**: 2.10.3
- **Plugins**: dialog, fs, clipboard, shell, notification, opener
- **Key Dependencies**: notify, portable-pty, rusqlite, tokio, reqwest

## Code Style Guidelines

### TypeScript
1. **Follow VSCode's existing patterns**: The codebase is ported from VSCode, maintain consistency
2. **Module imports**: Use `.js` extension for imports (ES modules requirement)
   ```typescript
   import { URI } from 'vs/base/common/uri.js';
   ```
3. **Service injection**: Use VSCode's DI pattern with `@inject` decorators
4. **Lifecycle management**: Implement `IDisposable` for cleanup

### Rust
1. **Command naming**: Use snake_case for command names
2. **Error handling**: Return `Result<T, String>` for Tauri commands
3. **State management**: Use Tauri's State<'_, Arc<T>>` for shared state
4. **Async operations**: Use `tokio` for async runtime

### File Organization
- VSCode's layered architecture is preserved
- Each layer has subdirectories: `common/` (pure TS), `browser/` (DOM), `node/` (native)
- Contributions are in `contrib/` directories
- Services are in `services/` directories

## Testing

### Current Testing Strategy
- VSCode's original test files are present in `test/` subdirectories
- Tests are excluded from TypeScript compilation (`tsconfig.json`)
- Integration tests would need to be run in a Tauri environment

### Manual Testing
1. Run `npm run tauri dev`
2. Test file operations (open folder, create/edit files)
3. Test terminal functionality
4. Test extensions from OpenVSX

## Rust Backend Commands

The following Tauri commands are available (defined in `src-tauri/src/lib.rs`):

### File System
- `read_file`, `read_file_bytes` - Read file contents
- `write_file`, `write_file_bytes` - Write file contents
- `read_dir` - List directory entries
- `stat` - Get file metadata
- `mkdir` - Create directories
- `remove` - Delete files/directories
- `rename` - Move/rename files
- `exists` - Check file existence

### Terminal
- `terminal_spawn` - Start a new PTY
- `terminal_write` - Write to PTY
- `terminal_resize` - Resize PTY
- `terminal_kill` - Kill PTY process
- `terminal_get_pid` - Get PTY process ID
- `get_default_shell`, `check_shell_exists`, `get_available_shells` - Shell management
- `get_shell_integration_dir`, `setup_zsh_dotdir` - Shell integration

### Search
- `search_files` - Find files by pattern
- `search_text` - Search file contents

### Window
- `create_window`, `close_window` - Window management
- `set_window_title` - Update window title
- `get_monitors` - Display information

### OS
- `get_os_info` - Operating system info
- `get_env`, `get_all_env` - Environment variables
- `get_shell` - Default shell

### Storage
- `storage_get`, `storage_set`, `storage_delete` - SQLite-backed key-value storage

### Git
- `git_status`, `git_diff`, `git_log`, `git_log_graph` - Git queries
- `git_add`, `git_commit`, `git_checkout` - Git operations
- `git_branches`, `git_create_branch`, `git_delete_branch` - Branch management
- `git_push`, `git_pull`, `git_fetch` - Remote operations
- `git_stash`, `git_reset`, `git_show` - Additional operations
- `git_init`, `git_is_repo`, `git_clone`, `git_remote_list`, `git_run`

### Extension Host
- `start_extension_host`, `stop_extension_host`, `extension_host_port`

### Network
- `fetch_url`, `fetch_url_text` - HTTP requests
- `proxy_request` - CORS proxy for OpenVSX

### Debug
- `debug_spawn_adapter`, `debug_send`, `debug_kill`, `debug_list_adapters`

### Tasks
- `task_spawn`, `task_kill`, `task_list`

## Important Notes for Development

### Global Variables
The frontend relies on several globals set in `index.html`:
- `_VSCODE_FILE_ROOT` - Base path for file resolution
- `_VSCODE_PRODUCT_JSON` - Product configuration
- `_VSCODE_PACKAGE_JSON` - Package metadata
- `__SIDEX_TAURI__` - Tauri detection flag

### Extension Gallery
Extensions are loaded from OpenVSX (open-vsx.org) instead of Microsoft's marketplace. The `fetch` API is intercepted in `index.html` to proxy OpenVSX requests through the Tauri backend to bypass CORS.

### Menu System
Native menus are built in Rust (`src-tauri/src/lib.rs`). Menu actions are forwarded to the frontend via `window.__sidex_menu_action()` and mapped to VSCode commands in `src/main.ts`.

### Storage
User data is stored in:
- SQLite database at `app_data_dir/sidex_storage.db` for settings/state
- File system for workspaces

### Security Considerations
- All file system operations go through Rust backend with proper path validation
- Extensions run in a separate extension host process
- No `contextBridge` needed (Tauri's security model differs from Electron)
- DevTools are enabled in all builds for debugging

## Dependencies to Know

### Frontend Key Dependencies
- `@tauri-apps/api` - Tauri JavaScript API
- `monaco-editor` - The text editor component
- `@xterm/xterm` + addons - Terminal functionality
- `vscode-textmate` + `vscode-oniguruma` - TextMate grammar support
- `@vscode/tree-sitter-wasm` - Tree-sitter parsing
- `@vscode/vscode-languagedetection` - Language detection

### Backend Key Dependencies
- `tauri` - Core Tauri framework
- `portable-pty` - Cross-platform PTY support
- `notify` - File system watching
- `rusqlite` - SQLite database
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `walkdir` - Directory traversal

## Common Development Tasks

### Adding a New Tauri Command
1. Add command function to appropriate file in `src-tauri/src/commands/`
2. Export from `src-tauri/src/commands/mod.rs`
3. Register in `src-tauri/src/lib.rs` in the `invoke_handler!` macro
4. Call from frontend via `import { invoke } from '@tauri-apps/api/core'`

### Adding a New VSCode Contribution
1. Create files in appropriate `contrib/` directory
2. Register contribution in the workbench main file
3. Follow VSCode's contribution patterns

### Modifying the UI
- Most UI styles are in CSS files throughout `src/vs/`
- Theme colors are defined in theme services
- The workbench layout is in `src/vs/workbench/browser/layout.ts`

## License

MIT (same as VSCode source)

## References

- [VSCode Source](https://github.com/microsoft/vscode) (MIT License)
- [Open Claw Methodology](https://github.com/instructkr/claw-code)
- [Tauri Documentation](https://tauri.app/)
- [Monaco Editor](https://microsoft.github.io/monaco-editor/)
