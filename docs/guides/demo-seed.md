# Demo Vault Seeding

The demo seeding script creates a realistic vault for demos or screenshots. It generates OS-appropriate entries, a small inbox, and a small snoozed queue.

## Script location
- `scripts/demo_seed.py`

## What it creates
- 100-250 entries (OS-specific mix of packages, apps, configs, scripts).
- Inbox items: 0-15 entries (default 12).
- Snoozed items: a small curated list.

The script favors realistic sources and command lines that match the detectors.

## Usage
```bash
python3 scripts/demo_seed.py --vault ~/DemoVault --inbox 12
```

## Notes
- `--inbox` is capped at 15 to keep the Inbox small for demos.
- The script does not run detectors; it writes entries directly to the vault.
- It will create `entries/` and `.state/` under the chosen vault path.
- The resulting vault is compatible with the TUI and CLI.

## Syncing with the app
- If you want the app to use the demo vault, set it in one of these ways:
  - TUI Settings tab: update the path and confirm switch/move.
  - CLI: `SETUPVAULT_PATH=~/DemoVault setupvault inbox`.
  - Config file: edit `~/.config/setupvault/config.yaml`.

## Resetting
To regenerate the demo vault, delete the vault directory and re-run the script.
