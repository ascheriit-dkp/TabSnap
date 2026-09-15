# ADR 0015 — Target Firefox 139+ with a separate manifest and adapter

## Status

Accepted.

## Context

TabSnap now has one shared Chromium implementation for Chrome and Microsoft Edge. Firefox implements the same WebExtensions model but differs in several places that matter to TabSnap:

- the full `tabGroups` API is available from Firefox 139
- Manifest V3 background service workers are not supported; Firefox uses background scripts/event pages
- Firefox MV3 signing requires Gecko-specific manifest metadata
- AMO requires a data-collection declaration for new submissions
- restoreable privileged/internal URLs differ from Chromium
- some tab-group UI semantics differ even when the stored group state is equivalent

Forking the `.tabsnap` format or cryptographic pipeline would not solve any of these differences.

## Decision

The first Firefox target is desktop Firefox 139 or newer.

Firefox gets a browser-specific manifest/package and a Firefox adapter/runtime boundary in M26. The snapshot schema, validation, serialization, compression, encryption, UI logic that is browser-neutral, and companion protocol remain shared.

The Firefox manifest will:

- use Manifest V3
- set `browser_specific_settings.gecko.strict_min_version` to `139.0`
- provide a stable Gecko extension ID before signing/release
- declare `data_collection_permissions.required` as `none`
- request exactly `tabs` and `tabGroups`
- keep `http://127.0.0.1/*` optional
- use `background.scripts` for `background.js`

Firefox restore will use a Firefox-specific URL policy and report skipped privileged URLs instead of removing them from snapshots.

TabSnap will preserve group collapsed state and active-tab intent, but it will not emulate Chromium's visual behavior when Firefox handles a collapsed group containing the active tab differently.

## Consequences

M26 can focus on browser API behavior instead of changing the file format or crypto.

Firefox 138 and older are intentionally unsupported by the first adapter because full tab-group metadata parity would require extra fallback behavior.

The Firefox release artifact cannot be byte-for-byte identical to the Chrome/Edge artifact because the manifest background model and Gecko signing metadata differ.

The production Gecko extension ID is intentionally deferred until the Firefox package/store work is implemented; the M25 fixture uses a non-release spike ID.
