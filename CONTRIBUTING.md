# Contributing to SideX

Thanks for your interest in contributing! SideX was released early specifically so the community can help build it out. Every contribution matters.

## Quick Start

```bash
git clone https://github.com/Sidenai/sidex.git
cd sidex
npm install
npm run tauri dev
```

See the [README](./README.md) for full prerequisite details.

## How to Contribute

1. **Fork** the repo
2. **Create a branch** — `git checkout -b my-fix`
3. **Make your changes** and test them with `npm run tauri dev`
4. **Submit a PR** with a clear description of what you changed and why

We'll review PRs as fast as we can. If your change gets merged, you'll be added as a contributor.

## What to Work On

Check the [Issues](https://github.com/Sidenai/sidex/issues) tab for open tasks. If you don't see an issue for what you want to fix, just open a PR — we're not strict about process.

### High-Impact Areas

- **Terminal** — Shell integration, profile detection, resize handling
- **Extensions** — Extension host stability, API coverage
- **File System** — Watcher reliability, large workspace support
- **Editor** — Language service integration, IntelliSense
- **Debug** — Debug adapter protocol
- **Settings** — Settings UI, keybinding editor
- **Search** — Workspace-wide search
- **Cross-Platform** — Windows and Linux testing
- **UI** — Layout bugs, theme gaps, accessibility

## Code Guidelines

### TypeScript
- Follow the existing VSCode patterns in the codebase
- Use `.js` extensions on imports (ES modules)
- Use VSCode's DI pattern with `@inject` decorators

### Rust
- Commands go in `src-tauri/src/commands/`
- Register new commands in `src-tauri/src/lib.rs`
- Use `Result<T, String>` for command return types
- Use `tokio` for async work

### General
- Keep PRs focused — one feature or fix per PR when possible
- If making a big architectural change, open an issue first to discuss

## Project Layout

- `src/vs/` — The VSCode workbench (TypeScript)
- `src-tauri/src/` — Rust backend replacing Electron
- `ARCHITECTURE.md` — Deep dive into how it all maps together
- `AGENTS.md` — Detailed guide for AI-assisted development

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
