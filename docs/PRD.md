# SetupVault Product Requirements Document

## 1. Purpose
SetupVault is a local-first system documentation tool that detects changes to a machine and captures the intent behind those changes. It prevents documentation drift by pairing automated reproduction commands with user-provided rationales, stored in a human-readable Markdown vault.

## 2. Problem statement
Developers routinely modify system state (install packages, edit configs, apply defaults) without recording why. This causes:
- Clean-install anxiety and brittle onboarding.
- Mysteryware and forgotten dependencies.
- Loss of agency from opaque automation.

## 3. Goals
- Capture system changes without interrupting user flow.
- Require a rationale for every entry.
- Keep all data local and human-readable.
- Provide a fast CLI for capture and a TUI for review.
- Work across macOS, Linux, and Windows.

## 4. Non-goals
- Cloud sync or collaboration.
- Full system provisioning replacement.
- Automated remediation of detected drift.

## 5. Target users
- Developers and power users maintaining complex local setups.
- Teams that want reproducible environments without centralized tooling.

## 6. Product principles
- Local-first and private.
- Calm technology: detection is passive, snooze is first-class.
- Intent-first: rationale required before approval.
- Unix philosophy: silence on success, clear errors on failure.

## 7. Functional requirements

### 7.1 Detection
- Detect package installs/changes via:
  - Homebrew
  - npm global
  - Cargo (Rust)
  - pip
  - apt (dpkg-query)
  - dnf/yum
  - pacman
  - flatpak
  - snap
  - winget
  - Chocolatey
  - Scoop
- Detect configuration changes via:
  - Dotfiles (e.g., `~/.zshrc`, `~/.gitconfig`, `~/.vimrc`)
  - macOS defaults
- Detect installed applications via:
  - macOS `/Applications`
  - Linux desktop entries
  - Windows Program Files
- Run detectors concurrently.
- Generate exact reproduction commands for each change.
- Run secret-safety heuristics before entry creation (warn only).

### 7.2 Capture and rationale
- Every entry requires a non-empty rationale.
- CLI supports quick capture with rationale flags.
- TUI supports batch review and manual capture flows.

### 7.3 Vault storage
- Local vault stored at `~/.setupvault/` by default.
- Override via `SETUPVAULT_PATH` or config preference.
- Entries stored as Markdown files with strict YAML frontmatter.
- Stable directory structure by entry type and source.

### 7.4 TUI experience
- Tabs: Dashboard, Inbox, Library, Snoozed, Settings.
- Manual capture flow in a dedicated modal.
- Filters and command palette for fast navigation.
- Prompts for destructive or high-impact actions.

### 7.5 CLI experience
- Silent on success.
- Clear error reporting on failure.
- Supports capture, list/search, approve/snooze/ignore, and export.

### 7.6 Settings and vault management
- Settings tab can switch to a new vault path or move the current vault.
- Changes prompt for confirmation.
- Configuration is persisted in `config.yaml`.

### 7.7 Metrics
- Vault health is computed from Inbox + Library only.
- Snoozed items are excluded from the health calculation.

## 8. Data model requirements

### 8.1 Entry fields
- `id`: UUID
- `title`: human-readable title
- `type`: package, config, application, script, or other
- `source`: detector or manual source label
- `cmd`: reproduction command
- `system`: OS and arch metadata
- `detected_at`: ISO 8601 timestamp
- `status`: active, ignored, snoozed
- `tags`: list of strings

### 8.2 Entry body
- `# Rationale` section required.
- `# Verification` section recommended.

## 9. Architecture requirements
- Rust workspace with clean boundaries.
- `sv-core` contains domain entities and rules; no IO or async.
- Infrastructure and interfaces live in separate crates.
- `thiserror` in libraries, `anyhow` at binaries.
- `clippy::pedantic` enforced.

## 10. Testing requirements
- Unit tests in `sv-core`.
- Integration tests in `sv-fs` using temp directories.
- Snapshot tests for CLI and TUI outputs using `insta`.

## 11. UX requirements
- Calm technology standards are mandatory.
- Notifications are subtle; snooze is first-class.
- No modal workflow blocks basic navigation.
- Errors are surfaced without halting workflows.

## 12. Success metrics
- Users can rebuild their environment using captured commands and rationales.
- Vault entries remain readable without the app.
- TUI workflows enable fast review and approval.
