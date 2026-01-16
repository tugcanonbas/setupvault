# ADR-001: Ratatui for TUI

## Status
Accepted

## Context
SetupVault requires a premium, responsive terminal UI with component-based rendering, event handling in a single render loop, and cross-platform support. The TUI should support complex layouts, keybinding hints, and overlays.

## Decision
Use `ratatui` as the TUI framework, paired with `crossterm` for terminal input/output.

## Consequences
- Strong ecosystem and active maintenance.
- Rendering model aligns with component-driven architecture.
- Requires explicit layout management and state-driven rendering, which fits the project's design goals.
