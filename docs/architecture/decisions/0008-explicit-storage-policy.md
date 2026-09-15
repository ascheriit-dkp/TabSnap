# ADR 0008 — Explicit Windows storage policy

## Status

Accepted for M17.

## Context

The portable companion must work from a normal directory or removable media without administrator privileges, while still allowing a user to keep snapshots in their Windows profile or another chosen directory.

Windows drive classification is not a reliable policy signal: external USB SSDs can be reported as fixed disks. Automatically switching storage based on drive type would therefore make portable behavior unpredictable.

## Decision

The companion has three explicit storage modes:

- `portable` — `snapshots/` next to the executable; default when no config exists
- `local` — `%LOCALAPPDATA%\TabSnap\snapshots`
- `custom` — an absolute directory selected by the user

The optional selection file is `tabsnap-companion.conf` next to the executable. No registry key is required.

Before persisting a mode change, the companion creates the target directory if necessary and verifies write access using a unique probe file that is removed immediately.

`GetDriveTypeW` is used only for an informational drive hint. Its result never changes the selected mode and never blocks portable storage.

## Consequences

A copied USB directory keeps its storage preference with the executable. With no config file it always falls back to portable mode.

Local mode deliberately depends on `LOCALAPPDATA` and fails clearly if that environment location is unavailable.

Custom paths are required to be absolute so saved behavior does not depend on the process working directory.

The configuration contains no passwords, snapshot payloads or browser-session data.
