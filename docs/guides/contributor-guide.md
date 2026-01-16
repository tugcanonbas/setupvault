# Contributor Guide

## Development principles
- Keep the domain pure: no IO or async in `sv-core`.
- Favor clarity over cleverness.
- Treat the vault format as a stable public API.

## Workspace layout
```
setupvault/
├── Cargo.toml
├── crates/
│   ├── sv-core
│   ├── sv-fs
│   ├── sv-detectors
│   ├── sv-tui
│   ├── sv-cli
│   └── sv-utils
├── scripts/
└── src/main.rs
```

## Build and run
- TUI: `cargo run`
- CLI: `cargo run -- <subcommand>`

Examples:
```bash
cargo run -- inbox --refresh
cargo run -- capture "jq" --rationale "JSON parsing" --entry-type package
```

## Coding standards
- `clippy::pedantic` is enforced.
- `thiserror` for library errors, `anyhow` for binaries.
- Every `pub` item requires `///` doc comments.
- ASCII-only comments unless the file already uses Unicode.

## Testing
- Unit tests live in `sv-core`.
- Integration tests in `sv-fs` (temp directories).
- Snapshot tests in `sv-cli` and `sv-tui` using `insta`.

Run tests:
```bash
cargo test -p sv-core -p sv-fs -p sv-cli -p sv-tui
```
Update snapshots:
```bash
INSTA_UPDATE=always cargo test -p sv-cli -p sv-tui
```

## Adding detectors
1) Implement `Detector` in `sv-detectors`.
2) Ensure the `name()` is stable and unique.
3) Return `DetectedChange` with correct `source`, `cmd`, and `EntryType`.
4) Add the detector to `default_detectors()` under the appropriate OS.
5) Update `docs/architecture/detectors.md`.

## Vault format changes
Any change to frontmatter fields or folder layout requires:
- Migration notes in `docs/architecture/data-storage.md`.
- A versioned ADR.
- Backwards compatibility if possible.

## TUI changes
- Keep interactions non-blocking.
- Prefer explicit confirmations for destructive or high-impact actions.
- Update `docs/architecture/tui-surface.md` for keybinding changes.
