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

### M23 — Chromium generalization

- separate Chromium runtime detection from capture/restore logic
- record Chrome and Edge source metadata correctly
- keep one shared Chromium implementation
- cover browser-specific internal URL handling

### M24 — Edge support

- Edge build/package
- capture/restore compatibility tests
- companion bridge verification
- Edge release artifact

### M25 — Firefox compatibility spike

- tabs/windows/groups capability matrix
- permissions and manifest differences
- restore behavior and unsupported features

### M26 — Firefox adapter

- implement Firefox-specific capture/restore behind the adapter boundary
- keep schema, crypto and companion protocol shared

### M27 — cross-browser compatibility

- Chrome ↔ Edge restore
- Chromium ↔ Firefox best-effort restore
- document feature-loss rules

### M28 — extension stores

- Chrome Web Store
- Microsoft Edge Add-ons
- Firefox Add-ons
- release/privacy/permission documentation

## M29-M34 — Level 4

Whole-machine multi-browser capture and restore.

### M29 — coordination foundation

- ephemeral registry of connected browser instances in the companion
- Chrome, Edge and Firefox runtime/capability metadata
- authenticated registration, heartbeat and listing over loopback
- lease expiry so stale browser instances disappear automatically
- no tab data, passwords or browser state persisted by the registry

### M30 — coordinated capture

- create whole-machine capture jobs from the companion
- dispatch capture work only to explicitly connected browser instances
- collect per-browser encrypted results with partial-failure reporting
- preserve the rule that the companion cannot decrypt browser workspaces

### M31 — machine snapshot container

- versioned Level 4 container for multiple encrypted browser snapshots
- machine/browser manifest with bounded metadata
- atomic storage and validation of the complete bundle
- forward-compatible handling of missing or unsupported browsers

### M32 — coordinated restore

- map stored browser entries to available Chrome, Edge and Firefox instances
- dispatch restore work to each target browser
- preserve cross-browser feature-loss rules
- explicit partial success, skip and retry semantics

### M33 — whole-machine UX

- companion view of connected browsers and capabilities
- one capture/restore flow across the machine
- progress, per-browser errors and retry controls
- no hidden discovery, background network access or cloud dependency

### M34 — Level 4 hardening

- multi-browser integration and race tests
- hostile container/protocol tests and resource limits
- threat-model and privacy documentation update
- portable release packaging and 1.0 readiness review

`1.0.0` waits until Level 4 is stable.
