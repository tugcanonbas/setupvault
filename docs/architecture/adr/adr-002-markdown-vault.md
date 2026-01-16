# ADR-002: Markdown Vault with YAML Frontmatter

## Status
Accepted

## Context
SetupVault must remain local-first, durable, and human-readable without application dependency. Entries should be easy to inspect, diff, and edit with standard tools.

## Decision
Store entries as Markdown files with strict YAML frontmatter in a deterministic directory structure.

## Consequences
- Users can access and edit entries with any text editor.
- Backups and version control are straightforward.
- Requires careful serialization and validation rules to enforce schema consistency.
