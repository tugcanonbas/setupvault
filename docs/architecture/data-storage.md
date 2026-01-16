# Data Storage Architecture

## Vault location
SetupVault stores all data locally.
- Default: `~/.setupvault/`
- Override: `SETUPVAULT_PATH` environment variable
- Persisted preference: `~/.config/setupvault/config.yaml` (or OS equivalent via `dirs::config_dir`)

The TUI Settings tab writes the config file automatically when you move or switch the vault.

## Directory layout
```
setupvault-vault/
├── .state/
│   ├── inbox.yaml
│   ├── snoozed.yaml
│   └── detectors/
│       ├── homebrew.yaml
│       ├── npm.yaml
│       └── ...
└── entries/
    ├── packages/
    │   ├── homebrew/
    │   ├── npm/
    │   └── ...
    ├── configs/
    ├── applications/
    ├── scripts/
    └── other/
```

## Storage flow diagram
```text
┌───────────────┐        ┌──────────────────────────────┐
│  Detection    │        │           Vault              │
│ (runtime)     │        │                              │
└──────┬────────┘        └──────────────┬───────────────┘
       │                               │
       ▼                               ▼
  .state/detectors/<source>.yaml   .state/inbox.yaml
                                       │
                                       ▼
                          Approve -> entries/<type>/<source>/*.md
                                       │
                                       ▼
                                  Library view
```

## Entry file format
Each entry is a Markdown file with YAML frontmatter. Filenames are deterministic and include source + title + UUID.

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

## Required frontmatter fields
- `id`, `title`, `type`, `source`, `cmd`, `system`, `detected_at`, `status`.
- `tags` is optional but encouraged.
- `# Rationale` is required and must be non-empty.
- `# Verification` is optional but recommended.

## State cache
`.state/` stores internal metadata:
- Inbox queue (`inbox.yaml`) for pending changes.
- Snoozed queue (`snoozed.yaml`).
- Detector snapshots in `.state/detectors/` for diffing.

The state directory is internal and can be rebuilt. Deleting `.state/` forces a fresh inbox refresh.

## Config file
`~/.config/setupvault/config.yaml` stores user preferences.
Current fields:
- `path`: optional custom vault path.

## Moving the vault
The TUI Settings tab supports two actions:
- Switch: points to a new vault path and initializes it if missing.
- Move: migrates the entire vault to a new path and updates config.

Both actions prompt for confirmation.
