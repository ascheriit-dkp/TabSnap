# ADR 0017 — Keep cross-browser restore best effort

## Status

Accepted.

## Context

The `.tabsnap` format is browser-agnostic, but browser APIs are not. Chrome, Edge and Firefox can all represent normal web tabs, windows, pinning and tab groups, while privileged/internal URLs and some UI semantics remain browser-specific.

Trying to make cross-browser restore look perfectly lossless would require rewriting URLs or inventing target-browser state that was not present in the source snapshot.

## Decision

Same-browser restore remains the fidelity baseline.

Cross-browser restore is best effort:

- preserve the original snapshot unchanged
- use the target browser's real restore policy
- report known non-portable tabs before restore
- restore every tab the target browser accepts
- report rejected tabs as warnings
- preserve canonical group/window/tab state without emulating another browser's UI
- never silently translate one browser's privileged URL into another URL

Compatibility assessment reuses the same browser-specific URL policies used by restore wherever possible.

## Consequences

Chrome ↔ Edge should normally preserve the full ordinary workspace, with browser-internal pages as the main exception.

Chromium ↔ Firefox preserves common web tabs and canonical workspace structure, while privileged URLs and some collapsed-group presentation remain browser-specific.

Future browser support must add explicit compatibility rules instead of changing the v1 file format merely to imitate browser UI details.
