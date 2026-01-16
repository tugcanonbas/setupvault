# User Manual

## Overview
SetupVault captures system changes and pairs each entry with a rationale. Entries are stored as Markdown in a local vault, so you can review or edit them without the app.

## Installation
SetupVault is currently run from source.

### Requirements
- Rust toolchain with Cargo.

### Build and run
```bash
cargo run
```
Running without a subcommand launches the TUI. The binary name is `setupvault`, while the CLI help uses `sv`.

## Vault initialization
The first time you run SetupVault, initialize a vault.
```bash
setupvault init
```

Optional custom path:
```bash
setupvault init --path ~/MyVault
```

## Vault location and configuration
- Default path: `~/.setupvault`
- Override with `SETUPVAULT_PATH`.
- Persisted preference in `~/.config/setupvault/config.yaml` (or OS equivalent).

The Settings tab also lets you switch or move the vault.

## CLI usage
The CLI is designed for fast capture and quick checks.

### Common commands
- `setupvault init --path <path>`: initialize a vault at a custom path.
- `setupvault capture`: create a manual entry (rationale required).
- `setupvault inbox`: list detected changes.
- `setupvault inbox --refresh`: run detectors and refresh the inbox.
- `setupvault approve <id>`: approve a detected change with rationale.
- `setupvault snooze <id>`: move a change to snoozed.
- `setupvault unsnooze <id>`: return a change to inbox.
- `setupvault ignore <id>`: discard a detected change.
- `setupvault list`: list entries in the library.
- `setupvault show <id>`: print a single entry.
- `setupvault search <query>`: search by title, tags, or rationale.
- `setupvault export <path>`: export entries to another directory.

### Capture flags
- `setupvault capture --rationale "<text>"` (required)
- `setupvault capture --entry-type <package|config|application|script|other>`
- `setupvault capture --source <label>` (default `manual`)
- `setupvault capture --cmd "<command>"`
- `setupvault capture --tag <tag>` (repeatable)
- `setupvault capture --verification "<text>"`

### Approve flags
- `setupvault approve --rationale "<text>"` (required)
- `setupvault approve --tag <tag>`
- `setupvault approve --verification "<text>"`

### Behavior
- Silent on success.
- Clear error on failure.

## TUI usage
The TUI is a dashboard for review and organization. It opens when you run `setupvault` with no subcommand.

### Tabs
- Dashboard: inbox count, managed items, vault health, top sources, recent activity.
- Inbox: detected changes waiting for action.
- Library: approved entries (search/filter + detail pane).
- Snoozed: deferred changes awaiting review.
- Settings: vault location and actions.

### Vault health
Vault health is calculated from Inbox + Library only. Snoozed items are excluded so deferrals do not reduce the health score.

### Filtering
- Press `/` to filter lists in Inbox, Library, or Snoozed.
- Press `Esc` to clear the filter.

### Manual capture
- Press `c` in any tab to create a manual entry.
- The modal collects title, rationale, command, tags, type, and verification.

### Settings tab
- `e`: edit the pending vault path.
- `a`: apply path and switch vault (initializes if missing).
- `m`: move the current vault to the new path.
- All changes prompt for confirmation before applying.

### Keybindings
Standard:
- Arrows: navigate
- Enter: open/select
- Space: toggle selection
- Esc: cancel or close
- Tab/Shift+Tab: cycle focus (in Inbox/Library/Snoozed)
- Home/End: start/end
- PageUp/PageDown: fast scroll

Power:
- j/k: list navigation
- h/l: pane or tab switch
- g/G: top/bottom
- Ctrl+u/Ctrl+d: half-page scroll
- a: accept
- s: snooze
- d: discard
- u: unsnooze (Snoozed tab)
- x: remove (Library/Snoozed)
- e: edit rationale (Library)
- r: refresh inbox
- c: manual capture
- ?: help
- p or : open command palette

## Demo vault
For demos, use the seeding script:
```bash
python3 scripts/demo_seed.py --vault ~/DemoVault --inbox 12
```
See `docs/guides/demo-seed.md` for details.

## Troubleshooting
- Detector failures do not block other detectors; check the status line for errors.
- To rebuild internal state, delete `.state/` in the vault.
- If the app keeps asking to initialize, confirm the vault path is set in Settings or `SETUPVAULT_PATH`.
