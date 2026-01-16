# TUI Architecture

## Goals
- Fast review and organization of detected changes.
- Calm defaults with power-user shortcuts.
- Always responsive, even when detectors fail.

## Architecture overview
- A single synchronous render loop drives the UI.
- Input events update a shared `App` state struct.
- Side effects (vault writes, detector runs) are invoked from event handlers.
- Rendering reads state only; no IO inside render functions.

## Core components
- `App` state
  - Active tab, focus, selection, filters, and input buffers.
  - Inbox and snoozed queues (`DetectedChange`).
  - Library entries (`Entry`).
  - Settings state (current path, pending path, pending confirmation).
- Views
  - Dashboard: stats, vault health, top sources, recent activity.
  - Inbox: action list for detected changes.
  - Library: searchable, editable entry list.
  - Snoozed: deferred queue.
  - Settings: vault path management.
- Popups
  - Rationale editor, filter input, command palette, settings path entry.
  - Confirmation modal for switch/move actions.
  - Manual capture flow (multi-step inputs).

## Event flow
1) Key events update `App` state.
2) Actions may perform filesystem writes or detector runs.
3) Errors update the status message without exiting the UI.
4) The next render pass displays the latest state.

## UI event loop diagram
```text
┌──────────────┐   key press   ┌──────────────┐
│   Terminal   │ ────────────> │ handle_key() │
└──────────────┘               └──────┬───────┘
                                     │ updates state
                                     ▼
                               ┌──────────┐
                               │   App    │
                               │  state   │
                               └────┬─────┘
                                    │
                     ┌──────────────┴──────────────┐
                     ▼                             ▼
               side effects                    render()
          (vault IO / detector)          (ratatui + layout)
                     │                             │
                     ▼                             ▼
               status message                terminal frame
```

## Data refresh
- Refresh (`r`) runs `default_detectors()` and diffs against snapshots.
- New changes are added to inbox, snapshots persisted per source.

## Error handling
- All IO is wrapped in `anyhow::Result` and reported in the status bar.
- Detector failures do not block the UI.

## Extensibility notes
- New tabs should define rendering and key handling explicitly.
- New actions should update both UI state and vault persistence in the same handler.
