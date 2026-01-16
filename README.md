# SetupVault

SetupVault is a local-first system documentation tool that detects software and configuration changes, then captures the intent behind those changes as Markdown entries. It is designed to reduce setup drift by pairing reproducible commands with human rationale.

## Why it exists
- Machines change every day: packages, apps, config edits, and defaults.
- Traditional notes drift quickly or never get written.
- SetupVault records what changed and why, in a format that survives without the app.

## Core concepts
- Vault: the local directory that stores all entries as Markdown with YAML frontmatter.
- Inbox: detected changes waiting for a rationale and decision.
- Library: approved entries that represent your documented system state.
- Snoozed: deferred changes that should resurface later.
- Manual capture: a first-class flow for entries you want to add yourself.

## Quick start (from source)
1) Build and run the TUI.
```bash
cargo run
```
2) Initialize a vault if prompted.
```bash
cargo run -- init
```
3) Refresh detection from the CLI.
```bash
cargo run -- inbox --refresh
```
4) Capture a manual entry from the CLI.
```bash
cargo run -- capture "jq" --rationale "JSON parsing for debugging scripts" \
  --entry-type package --source manual --cmd "brew install jq" --tag cli --tag json
```

Notes:
- The binary name is `setupvault`, but the CLI help uses the name `sv`.
- Running with no subcommand launches the TUI.

## Vault location
- Default: `~/.setupvault`
- Override with `SETUPVAULT_PATH` or in the TUI Settings tab.
- Stored preference: `~/.config/setupvault/config.yaml` (or OS equivalent via `dirs::config_dir`).

## What SetupVault detects
Detectors run per operating system and are intentionally conservative. Each detected change includes a reproduction command.

macOS:
- Homebrew formulae and casks
- macOS defaults
- `/Applications` (app bundle detection)
- npm global packages, cargo-installed crates, pip packages
- Dotfiles: `~/.zshrc`, `~/.gitconfig`, `~/.vimrc`

Linux:
- apt (dpkg-query), dnf, yum, pacman
- flatpak, snap
- Desktop applications from `.desktop` files
- npm, cargo, pip
- Dotfiles

Windows:
- winget (including Store entries), chocolatey, scoop
- Program Files
- npm, cargo, pip

## Documentation map
- `docs/guides/user-manual.md` - end user workflow and keybindings.
- `docs/guides/contributor-guide.md` - development workflows and standards.
- `docs/architecture/system-overview.md` - crate responsibilities and data flow.
- `docs/architecture/data-storage.md` - vault structure and file formats.
- `docs/architecture/detectors.md` - detectors per OS and sources.
- `docs/architecture/cli-surface.md` / `docs/architecture/tui-surface.md` - UI contract.
- `docs/guides/demo-seed.md` - demo vault seeding script.

## Repository layout
```
setupvault/
├── crates/
│   ├── sv-core        # domain models and rules
│   ├── sv-fs          # filesystem persistence and config
│   ├── sv-detectors   # detection sources
│   ├── sv-cli         # CLI interface
│   ├── sv-tui         # terminal UI
│   └── sv-utils       # shared helpers
├── scripts/           # demo seed and utility scripts
├── docs/              # documentation
└── src/main.rs        # binary entrypoint
```

## License
MIT

## Maintainer
Tuğcan ÖNBAŞ - tgcn@tugcanonbas.com
