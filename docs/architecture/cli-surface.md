# CLI Command Surface

## Goals
- Fast, scriptable workflows for capture and retrieval.
- Silent on success, actionable errors on failure.
- Compatible with shell pipelines.

## Command overview
- `init` — initialize a vault (optional path).
- `capture` — create a manual entry with required rationale.
- `inbox` — list detected changes (optionally refresh).
- `approve` — approve a detected change by id.
- `snooze` — defer a detected change by id.
- `unsnooze` — restore a snoozed change to the inbox.
- `ignore` — discard a detected change by id.
- `list` — list all entries.
- `show` — show a single entry as Markdown.
- `search` — search entries by title, tags, or rationale.
- `export` — export entries to a directory.

## Examples
Initialize:
```bash
setupvault init --path ~/SetupVault
```
Refresh inbox:
```bash
setupvault inbox --refresh
```
Capture a manual entry:
```bash
setupvault capture "ripgrep" --rationale "Fast code search" \
  --entry-type package --source manual --cmd "brew install ripgrep"
```
Approve an inbox item:
```bash
setupvault approve <id> --rationale "Needed for log parsing" --tag cli
```
Export entries:
```bash
setupvault export ~/Desktop/setupvault-export
```

## Flag conventions
Capture:
- `--rationale` (required)
- `--entry-type <package|config|application|script|other>`
- `--source <label>` (default `manual`)
- `--cmd <command>`
- `--tag <tag>` (repeatable)
- `--verification <text>`

Approve:
- `--rationale` (required)
- `--tag <tag>`
- `--verification <text>`

Inbox:
- `--refresh` runs detectors before listing

## Output format
- Inbox prints tab-separated rows: `id`, `title`, `source`, `cmd`.
- Silent on success for other commands.
- Errors return non-zero exit codes.
