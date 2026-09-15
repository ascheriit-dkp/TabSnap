# ADR 0016 — Share capture/restore through a configurable WebExtension adapter

## Status

Accepted.

## Context

Chrome, Edge and Firefox 139+ expose the browser primitives TabSnap needs: tabs, normal windows and tab groups. The important differences are runtime metadata, restoreable privileged URLs, a small set of window-state semantics and package metadata.

Copying the Chromium capture/restore implementation into a Firefox source tree would duplicate the highest-risk state reconstruction code and make browser behavior drift over time.

## Decision

TabSnap uses one shared WebExtension capture/restore engine configured by thin browser adapters.

The shared engine owns:

- window/tab capture and canonical ordering
- snapshot-local group IDs
- bounds, focus, pinned and active state
- non-destructive window/tab restore
- group reconstruction
- partial-failure reporting

A browser adapter supplies:

- runtime/source metadata detection
- restore URL resolution
- captured window-state normalization

Chromium keeps its existing Chrome/Edge runtime detector and privileged-URL behavior. Firefox has a separate runtime detector and restore policy, including native `about:newtab` creation and `docked` → `normal` normalization.

The application-facing `browser.ts` module is only a runtime dispatcher. Schema, crypto, compression and companion code remain browser-neutral.

## Consequences

Fixes to ordering, grouping, pinning, focus and partial restore apply to all supported browsers once.

Browser-specific differences stay small and directly testable instead of appearing as scattered `if (firefox)` branches.

The Firefox package still needs its own Manifest V3 file because Firefox uses `background.scripts` and Gecko-specific signing metadata rather than Chromium's service-worker manifest.
