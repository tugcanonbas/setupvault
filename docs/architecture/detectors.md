# Detectors

Detectors are the system scanners that produce `DetectedChange` records. Each detector has a stable `source` label and builds a reproduction command so entries can be replayed manually.

## Detector list by OS

### macOS
- `homebrew`
  - Formulae: `brew list --formula` => `brew install <name>`
  - Casks: `brew list --cask` => `brew install --cask <name>`
- `mac_defaults`
  - macOS defaults via `defaults read` per domain.
- `applications`
  - `/Applications` bundles, normalized for duplication with Homebrew casks.
- `dotfiles`
  - `~/.zshrc`, `~/.gitconfig`, `~/.vimrc`.
- `npm`, `cargo`, `pip`
  - Global package lists.

### Linux
- `apt`
  - `dpkg-query -W` (apt package database).
- `dnf`, `yum`
  - `dnf list installed`, `yum list installed` with rpm parsing.
- `pacman`
  - `pacman -Q` output parsing.
- `flatpak`
  - `flatpak list`.
- `snap`
  - `snap list`.
- `applications`
  - `.desktop` files from `/usr/share/applications` and `~/.local/share/applications`.
- `dotfiles`, `npm`, `cargo`, `pip`.

### Windows
- `winget`
  - `winget list` installed packages.
- `msstore`
  - Microsoft Store entries parsed from `winget list`.
- `chocolatey`
  - `choco list --local-only`.
- `scoop`
  - `scoop list`.
- `applications`
  - Program Files (both 64-bit and 32-bit roots).
- `npm`, `cargo`, `pip`.

## Source and type mapping
- Package managers (brew, apt, etc.) emit `EntryType::Package`.
- App folders and desktop entries emit `EntryType::Application`.
- Dotfiles and defaults emit `EntryType::Config`.

## Snapshot and diff strategy
Detectors are idempotent and stateless. The CLI/TUI:
- Store a per-source snapshot in `.state/detectors/<source>.yaml`.
- Diff current results against the snapshot by `(source, title)`.
- Append new changes to the inbox.

## Detector flow diagram
```text
┌──────────────────┐
│ Detector (scan)  │
└────────┬─────────┘
         │
         ▼
 [DetectedChange list]
         │
         ▼
Load snapshot: .state/detectors/<source>.yaml
         │
         ▼
Diff by (source, title)
         │
         ├───────────────┐
         ▼               ▼
 Save new snapshot    Append new items
    (same path)      to .state/inbox.yaml
```

## Error behavior
A single detector failure does not block the overall scan. Errors are surfaced in CLI/TUI status output while other detectors still contribute results.
