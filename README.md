
# SetupVault

**Local-first system documentation** that captures what changed on your machine and why. SetupVault turns installs, config edits, and defaults into human-readable Markdown entries with reproducible commands.

![SetupVault cover](images/setupvault_cover.png)

## What is SetupVault?
SetupVault is a CLI + TUI app that detects software and configuration changes across macOS, Linux, and Windows. It collects changes into an Inbox, prompts for rationale, and stores approved entries in a durable Markdown vault.

## Problem it solves
- **Documentation drift**: installs and tweaks happen faster than anyone writes them down.
- **Lost intent**: commands without rationale are hard to trust later.
- **Rebuild pain**: new machines become archaeology projects.

## Who it’s for
- Developers and power users who maintain complex local setups.
- Teams that want reproducible environments without centralized tooling.
- Anyone who wants a local-first audit trail of system changes.

## Features

### Core functionality
- **Cross-platform detection** with OS-specific sources.
- **Inbox workflow** for review, snooze, or ignore.
- **Manual capture** for anything detectors miss.
- **Rationale-first** entries stored as Markdown + YAML frontmatter.
- **Export** entries to a folder for sharing or backup.

### Detectors by OS
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

### Vault management
- Default vault: `~/.setupvault`
- Override with `SETUPVAULT_PATH` or the TUI Settings tab
- Safe move or switch with confirmation prompts
- Persistent config at `~/.config/setupvault/config.yaml` (or OS equivalent)

## How it works
1) Detectors scan the system and emit `DetectedChange` items.
2) The CLI/TUI diffs results against snapshots to find new changes.
3) New changes enter the Inbox.
4) Approval requires a rationale and creates a Markdown entry.
5) Snoozed items stay out of the Vault Health metric.

## Installation
SetupVault is currently run from source.

### Requirements
- Rust toolchain with Cargo
- Python 3 (only for the demo seed script)

### Build and run
```bash
cargo run
```

Run a CLI command:
```bash
cargo run -- inbox --refresh
```

## Usage

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

## Demo vault
Use the demo seed script to generate a realistic demo vault:
```bash
python3 scripts/demo_seed.py --vault ~/DemoVault --inbox 12
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

## Documentation map
- `docs/guides/user-manual.md` - end user workflow and keybindings
- `docs/architecture/system-overview.md` - data flow and crate responsibilities
- `docs/architecture/detectors.md` - detector details by OS
- `docs/architecture/data-storage.md` - vault layout and file formats
- `docs/architecture/tui-architecture.md` - UI event loop
- `docs/architecture/cli-surface.md` - CLI contract
- `docs/guides/demo-seed.md` - demo vault seeding

## License
MIT

## Maintainer
Tuğcan ÖNBAŞ - tgcn@tugcanonbas.com
