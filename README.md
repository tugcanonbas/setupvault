# SetupVault

**Local-first system documentation** — capture installs, configuration changes, and defaults with the *why* attached. SetupVault turns system drift into Markdown entries you can trust, review, and replay.

![SetupVault cover](https://raw.githubusercontent.com/tugcanonbas/setupvault/main/images/setupvault_cover.png)

## What is SetupVault?

SetupVault is a CLI + TUI app that detects software and configuration changes across macOS, Linux, and Windows. It collects changes into an Inbox, prompts for rationale, and stores approved entries as durable Markdown files with YAML frontmatter.

### Problem it solves

- **Documentation drift**: machines evolve faster than human notes.
- **Lost intent**: commands without rationale are hard to trust later.
- **Rebuild pain**: new machines become archaeology projects.
- **Tool sprawl**: installs and defaults scatter across package managers and settings.

### Who it’s for

- **Developers & power users**: keep your environment reproducible and explainable.
- **Teams**: share setup intent without central provisioning.
- **Auditors**: maintain a local-first history of system changes.

## Features

### Core functionality

- **Cross-platform detection** with OS-specific sources.
- **Inbox workflow** to review, snooze, or ignore changes.
- **Rationale-first entries** stored as Markdown + YAML frontmatter.
- **Manual capture** for anything detectors miss.
- **Export** entries to a folder for sharing or backup.

### Detection sources by OS

macOS:
- Homebrew (formulae + casks)
- macOS defaults
- `/Applications`
- npm, cargo, pip
- Dotfiles (`~/.zshrc`, `~/.gitconfig`, `~/.vimrc`)

Linux:
- apt (dpkg-query), dnf, yum, pacman
- flatpak, snap
- `.desktop` applications
- npm, cargo, pip
- Dotfiles

Windows:
- winget (including Microsoft Store), chocolatey, scoop
- Program Files
- npm, cargo, pip

### User experience

- **Calm UI**: silent success, clear failures, no spam.
- **Snooze-first** workflows for deferred review.
- **Stable sorting** for dashboards and source charts.
- **Settings tab** for safe vault move/switch with confirmation.

## Installation

SetupVault is currently run from source.

### Prerequisites

- Rust toolchain with Cargo
- Python 3 (only needed for the demo seed script)

### Build and run

```bash
cargo run
```

Run a CLI command:

```bash
cargo run -- inbox --refresh
```

## How to use

### Basic flow

1) **Initialize a vault** if prompted:
   ```bash
   setupvault init
   ```
2) **Detect changes**:
   ```bash
   setupvault inbox --refresh
   ```
3) **Review in the TUI**:
   ```bash
   setupvault
   ```
4) **Approve** changes with a rationale.

### CLI highlights

- `setupvault init --path <path>`
- `setupvault inbox --refresh`
- `setupvault capture "jq" --rationale "JSON parsing" --entry-type package`
- `setupvault approve <id> --rationale "Needed for debugging"`
- `setupvault export <path>`

### TUI highlights

- Tabs: Dashboard, Inbox, Library, Snoozed, Settings
- Manual capture: press `c`
- Refresh detectors: `r`
- Help overlay: `?`

### Vault location

- Default: `~/.setupvault`
- Override: `SETUPVAULT_PATH` or TUI Settings
- Persisted preference: `~/.config/setupvault/config.yaml` (or OS equivalent)

## Demo vault

Generate a realistic demo vault for screenshots or demos:

```bash
python3 scripts/demo_seed.py --vault ~/DemoVault --inbox 12
```

## Vault format

Entries are Markdown files with YAML frontmatter:

```markdown
---
id: "550e8400-e29b-41d4-a716-446655440000"
title: "jq"
type: "package"
source: "homebrew"
cmd: "brew install jq"
system:
  os: "macos"
  arch: "arm64"
detected_at: "2023-10-27T10:00:00Z"
status: "active"
tags:
  - "cli"
  - "json"
---

# Rationale
Useful for parsing JSON responses in daily API debugging scripts.

# Verification
Run `jq --version` to check installation.
```

## Project structure

```
setupvault/
├── crates/
│   ├── sv-core        # domain models and rules
│   ├── sv-fs          # filesystem persistence and config
│   ├── sv-detectors   # detection sources
│   ├── sv-cli         # CLI interface
│   ├── sv-tui         # terminal UI
│   └── sv-utils       # shared helpers
├── scripts/           # demo seed and utilities
├── docs/              # documentation
└── src/main.rs        # binary entrypoint
```

### Key components

- **sv-core**: domain models and validation.
- **sv-fs**: vault IO, config, snapshots, and state queues.
- **sv-detectors**: OS-specific detectors and parsing.
- **sv-cli**: CLI surface; launches the TUI by default.
- **sv-tui**: terminal UI, inbox review, and settings.

## Limitations

- **Detection is best-effort**: missing binaries mean missing detectors.
- **No cloud sync**: vaults are local by design.
- **No auto-remediation**: SetupVault documents changes, it does not reverse them.

## Development

### Useful commands

```bash
cargo test -p sv-core -p sv-fs -p sv-cli -p sv-tui
INSTA_UPDATE=always cargo test -p sv-cli -p sv-tui
```

### Docs

- `docs/guides/user-manual.md` - end user workflow and keybindings
- `docs/architecture/system-overview.md` - data flow and crate responsibilities
- `docs/architecture/detectors.md` - detector details by OS
- `docs/architecture/data-storage.md` - vault layout and file formats
- `docs/architecture/tui-architecture.md` - UI event loop
- `docs/architecture/cli-surface.md` - CLI contract
- `docs/guides/demo-seed.md` - demo vault seeding

## Troubleshooting

### Detectors not returning results
- Ensure the underlying tool exists (e.g., `brew`, `apt`, `winget`).
- Run `setupvault inbox --refresh` and check for error output.

### TUI keeps asking to initialize
- Verify `SETUPVAULT_PATH` or update the Settings tab path.
- Confirm the vault directory contains `entries/` and `.state/`.

### Export not working
- Confirm the target directory exists and is writable.
- Use absolute paths to avoid shell expansion issues.

## License

MIT License

## Maintainer

Tuğcan ÖNBAŞ - tgcn@tugcanonbas.com

Website: https://tugcanonbas.com
