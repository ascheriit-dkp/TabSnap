# ADR 0013 — Chromium runtime detection

## Status

Accepted.

## Context

Chrome and Microsoft Edge expose the Chromium extension APIs used by TabSnap. Keeping separate capture and restore implementations would duplicate the same window, tab and tab-group logic.

The existing adapter was already API-compatible with Chromium browsers, but it always wrote `source.browser: chrome` and extracted only the `Chrome/...` user-agent version. In Edge, that would mislabel the snapshot and record the Chromium engine token instead of the Edge version.

## Decision

Keep one Chromium capture/restore implementation and detect the runtime separately.

The runtime detector:

- checks `Edg/<version>` before the shared `Chrome/<version>` token
- records Edge snapshots with `source.browser: edge`
- records Chrome snapshots with `source.browser: chrome`
- keeps generic Chromium builds on the existing `chrome` schema value until a distinct schema value is justified
- does not alter the `.tabsnap` format version

The adapter continues to use the `chrome.*` namespace because that is the Chromium extension API surface used by both Chrome and Edge.

Restore filtering blocks both `chrome-extension:` and `edge-extension:` URLs. Browser-owned pages such as `chrome://` and `edge://` remain best-effort: TabSnap lets the target browser attempt them and reports failures instead of silently deleting them.

## Consequences

Chrome and Edge share one implementation for capture, restore, groups, window state and companion integration.

M24 can package and test Edge without forking the core browser code.

Firefox remains a separate adapter because its WebExtension behavior and capabilities require an explicit compatibility pass.
