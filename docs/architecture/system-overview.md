# System Overview

SetupVault is a Rust workspace that separates the domain model from IO and UI. The core idea is to keep detection, storage, and user interaction loosely coupled while sharing a stable data model.

## Crate responsibilities
- `sv-core`
  - Domain types: `Entry`, `DetectedChange`, `Rationale`, `Tag`, `SystemInfo`.
  - Validation rules (rationale required, tag validation).
  - Traits for persistence (`VaultRepository`) and detection (`Detector`).
- `sv-fs`
  - Filesystem-backed vault implementation (`FsVault`).
  - Markdown + YAML frontmatter serialization.
  - Inbox/snoozed state queues and detector snapshots.
  - Vault path resolution and config persistence.
- `sv-detectors`
  - OS-specific change detectors.
  - Command execution and parsing of package manager output.
  - `default_detectors()` to select the correct detector list by OS.
- `sv-cli`
  - CLI surface (`sv` help name, binary is `setupvault`).
  - Capture, inbox refresh, approve/snooze/ignore, list/search, export.
  - TUI launch if no subcommand is provided.
- `sv-tui`
  - Terminal UI using `ratatui` + `crossterm`.
  - State-driven rendering loop and input handling.
  - Inbox, Library, Snoozed, Settings, and manual capture flows.
- `sv-utils`
  - Utility helpers shared across crates.

## Data flow (high level)
1) Detectors scan the system and return `DetectedChange` items.
2) The CLI/TUI groups changes by source and compares them to detector snapshots.
3) New changes enter the inbox queue (`.state/inbox.yaml`).
4) Approvals convert `DetectedChange` into `Entry` Markdown files.
5) Snoozed items remain in `.state/snoozed.yaml` until restored.

## Data flow diagram
```text
┌────────────────────┐      ┌────────────────────┐
│   sv-detectors     │      │   sv-cli / sv-tui  │
│ (scan + parse)     │      │ (group + diff)     │
└─────────┬──────────┘      └─────────┬──────────┘
          │                           │
          ▼                           ▼
  [DetectedChange]            .state/detectors/<source>.yaml
          │                           │
          └─────────────┬─────────────┘
                        ▼
                .state/inbox.yaml
                        │
                        ▼
                User approves
                        │
                        ▼
                entries/<type>/<source>/<file>.md
                        │
                        ▼
                 Library (approved)
```

Note: For detector-specific behavior and diffing, see `docs/architecture/detectors.md`.

## Key workflows
- Detection refresh (CLI or TUI):
  - Run `default_detectors()` concurrently.
  - Diff against snapshots per source.
  - Persist snapshots and append unique changes to inbox.
- Approval:
  - User supplies rationale, tags, optional verification.
  - Entry is validated and written as Markdown.
  - Inbox item is removed.
- Manual capture:
  - User provides title, rationale, command, tags, type, verification.
  - Entry is created directly in the library.
- Vault path changes:
  - Settings tab can switch to a new path (initializes if missing).
  - Move option migrates the entire vault contents.
  - Changes are persisted in `config.yaml` and reflected on next launch.

## Error handling
- `sv-core` returns domain errors only.
- `sv-fs` wraps IO errors as storage errors.
- CLI and TUI surface errors without panics; the TUI keeps the UI responsive.

## Supported operating systems
SetupVault branches the detector list per OS (macOS, Linux, Windows) using `std::env::consts::OS`. See `docs/architecture/detectors.md` for details.
