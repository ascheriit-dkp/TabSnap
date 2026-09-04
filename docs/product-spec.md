# Product spec

## Goal

Capture a browser workspace on one machine and restore it on another.

No account. No backend. No cloud storage.

## Level 1

One browser at a time.

Capture:

- windows
- window order and state
- window bounds when available
- tabs
- tab order
- active tab
- pinned tabs
- tab groups when supported

Export:

- encrypted copyable string
- encrypted `.tabsnap` file

Restore:

- validate first
- preview first
- restore without closing the current workspace by default
- skip unsupported tabs without killing the whole restore

## Data we do not capture

- cookies
- history
- page content
- form values
- local storage
- login sessions
- screenshots

Tab titles may be stored as optional metadata. They are not required for restore.

## Security rules

- snapshot data stays local
- no telemetry
- no analytics
- no remote fonts or CDN dependencies in the extension
- passwords are never stored
- imported snapshots are untrusted input
- crypto is versioned
- no custom cryptography

## Later

Level 2 adds a portable companion app.

Level 3 adds Edge and Firefox.

Level 4 adds whole-machine multi-browser capture and restore.
