# ADR 0022 — Explicit whole-machine companion UX

## Status

Accepted for Level 4 M33.

## Context

M29 through M32 provide the browser registry, coordinated capture, machine snapshot container and coordinated restore primitives. Those milestones expose diagnostic CLI flows but do not provide one end-user workflow that shows which browsers are participating, the progress of each target or which restore failures can be retried.

The UX must preserve the existing privacy boundary. Starting the desktop companion must not discover browsers automatically, scan profiles, contact the Internet or make the local protocol permanently active.

## Decision

The existing portable Win32 companion UI becomes the Level 4 whole-machine control surface. No WebView, installer, service, browser-process scanner or additional runtime is introduced.

The local protocol remains stopped when the UI opens. The user must press `Start protocol`, copy the generated pairing code and explicitly pair each browser page they want to participate.

Before the protocol thread is moved into its server loop, the UI keeps cloneable local control handles for:

- the ephemeral browser registry view;
- coordinated capture;
- coordinated restore.

Those handles share only the existing bounded in-memory registry/job stores. The UI never calls its own HTTP protocol.

A Win32 timer refreshes the local browser view and active operation status. It does not perform network discovery. Connected browsers are displayed with browser kind, bounded version, advertised capture/restore capabilities and a shortened ephemeral instance identifier.

The UI presents separate browser-snapshot and machine-snapshot lists. Machine restore choices come from strict `.tabsnap-machine` inspection; invalid, oversized, non-regular and symbolic-link entries are not offered.

Whole-machine capture is one action:

1. create a coordinated capture job for currently connected capture-capable browsers;
2. show per-browser pending/capturing/complete/failure state;
3. when terminal, atomically persist one machine container if at least one encrypted browser payload succeeded;
4. refresh the machine-snapshot list.

Whole-machine restore is one action:

1. select a machine snapshot;
2. create the M32 restore job against currently connected restore-capable browsers;
3. show source-to-destination routing and per-target state;
4. retain a retry control only when failed or browser-unavailable targets remain.

Retry reuses M32 semantics: completed restore targets are not replayed.

Storage changes are rejected after the local protocol has started in the current process. This prevents the UI's library root from diverging from the machine library and controls owned by the running protocol session.

## Privacy boundary

The companion UI may display:

- local file names and sizes;
- browser kind/version/capabilities;
- shortened ephemeral instance identifiers;
- bounded capture/restore state and failure codes.

It never receives or displays snapshot passwords, encryption keys, decrypted workspaces, URLs, tab titles or raw browser exceptions.

The UI timer only reads local in-memory state created after explicit pairing. It does not scan browser processes, enumerate browser profiles, access LAN services or contact cloud services.

## Consequences

- Level 4 capture and restore can be driven from one native portable window.
- Browser participation is visible and explicit.
- Partial success and retry behavior are visible per browser.
- Existing single-browser library and storage workflows remain available.
- The companion stays local-first and opaque to encrypted browser contents.
- M34 can focus on hostile-state hardening, regression coverage and release preparation rather than basic UX orchestration.
