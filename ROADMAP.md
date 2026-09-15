# Roadmap

## M0-M3 — foundation

- product spec
- monorepo
- tooling
- CI
- README
- docs
- GitHub Pages workflow

## M4-M6 — snapshot core

- `.tabsnap` schema
- validation
- serialization
- compression
- password KDF
- authenticated encryption

## M7-M9 — Chrome extension

- capture
- restore
- groups
- window geometry
- encrypted string export/import
- `.tabsnap` file export/import
- preview and errors

## M10-M15 — Level 1 hardening

- network audit
- permission audit
- threat model
- unit tests
- browser integration tests
- alpha
- beta

## M16-M22 — Level 2

Portable Windows companion.

### M16 — portable foundation

- native Windows executable
- no installer, service or administrator requirement
- derive portable storage from the executable directory
- Windows CI build and tests

### M17 — storage policy

- portable, local and custom storage modes
- removable-drive hints without assuming every USB device reports as removable
- writable-path validation and clear errors

### M18 — snapshot library

- safe `.tabsnap` file discovery
- import/copy/export operations
- atomic writes and collision-safe names
- opaque encrypted files remain encrypted at rest

### M19 — local companion protocol

- versioned localhost protocol
- explicit session authentication
- loopback-only binding
- no Internet-facing listener

### M20 — Chrome bridge

- opt-in extension connection to the local companion
- send and receive encrypted snapshots
- preserve extension-only operation when the companion is absent

### M21 — companion UI

- native snapshot library view
- storage-mode controls
- import/export and copy paths
- clear connection and privacy state

### M22 — Level 2 hardening

- Windows integration tests
- portable ZIP + checksum
- threat-model update
- alpha/beta release of the companion

## M23-M28 — Level 3

Edge, then Firefox.

## M29-M34 — Level 4

Whole-machine multi-browser capture and restore.

`1.0.0` waits until Level 4 is stable.
