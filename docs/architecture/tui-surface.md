# TUI Interaction Surface

## Navigation
- Tabs: Dashboard, Inbox, Library, Snoozed, Settings.
- Arrow keys and hjkl for navigation.
- Tab / Shift+Tab to switch focus between panes (Inbox/Library/Snoozed).
- `?` opens the help overlay.

## Core actions
Inbox:
- Accept (`a`) -> requires rationale
- Snooze (`s`)
- Ignore (`d`)
- Refresh (`r`) to run detectors

Snoozed:
- Unsnooze (`u`)
- Remove (`x`)

Library:
- Edit rationale (`e`)
- Remove (`x`)

Settings:
- Edit path (`e`)
- Apply and switch (`a`)
- Move vault (`m`)

Global:
- Manual capture (`c`)
- Command palette (`p` or `:`)

## Overlays and popups
- Help overlay with context-aware key hints.
- Input popups for rationale, filters, and settings path changes.
- Confirmation popup for switch/move actions.

## Filtering
- Press `/` to filter entries in Inbox/Library/Snoozed.
- `Esc` clears the current filter.

## Update flow
- UI updates optimistically after actions.
- Storage writes are executed immediately; errors appear in the status area.

## Vault health
The dashboard vault health metric uses Inbox + Library counts and excludes Snoozed entries.
