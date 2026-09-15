# ADR 0011 — Opt-in Chrome companion bridge

## Status

Accepted for M20.

## Context

M19 added an authenticated loopback protocol to the optional Windows companion. The Chrome extension now needs a way to use that protocol without turning the companion into a requirement, weakening the existing local-only browser workflow, or exposing plaintext workspace data to the native process.

Chrome extension pages need host permission for cross-origin `fetch()` calls. Because companion support is optional, granting localhost access at install time would give every user a permission that many users never need.

The M19 pairing code contains a per-process 256-bit session token. Persisting that token in extension storage would unnecessarily extend the lifetime of a credential that is designed to die with the companion process.

## Decision

The Chrome companion bridge is explicitly opt-in.

The manifest declares only:

```json
{
  "optional_host_permissions": ["http://127.0.0.1/*"]
}
```

The extension requests that permission only after the user presses **Connect** in the companion section. Required Chrome permissions remain unchanged.

A pairing code must match the exact v1 shape:

```text
tabsnap-companion:v1:<port>:<64-lowercase-hex-token>
```

The extension constructs the endpoint itself as `http://127.0.0.1:<port>`. No hostname, URL, scheme or path from user input is accepted, so the pairing field cannot turn the bridge into a general network client.

The session token is held only by the in-memory `CompanionClient` instance. It is not written to `chrome.storage`, `localStorage`, IndexedDB, the snapshot library, logs or configuration files. Closing/reloading the extension page therefore forgets the pairing and requires a fresh pairing action.

The extension performs all snapshot cryptography exactly as it does in Level 1:

```text
TabSnapSnapshot
  -> encryptSnapshot(password)
  -> opaque encrypted bytes
  -> loopback companion
```

and in the opposite direction:

```text
loopback companion
  -> opaque encrypted bytes
  -> decryptSnapshot(password)
  -> TabSnapSnapshot
```

The password is never included in an HTTP request. The companion never receives plaintext snapshot JSON.

The bridge uses the M19 Bearer token on every real operation, finite request timeouts, `credentials: omit`, `cache: no-store`, and `redirect: error`. Responses are type-checked and snapshot bodies remain subject to the existing 65 MiB limit.

## Extension-only fallback

Companion state is optional and isolated from the existing capture/export/import/restore controls. If the companion is absent, stopped, unpaired, denied localhost permission, or disconnected, all Level 1 workflows continue to work exactly as before.

Disconnecting destroys the in-memory client/token. Chrome may retain the user-granted optional localhost permission, but that permission alone does not authenticate to the companion; every M19 operation still requires the current per-process Bearer token.

## Security audit

The extension audit is updated narrowly:

- required permissions remain exactly `tabs` and `tabGroups`;
- the only optional host permission is `http://127.0.0.1/*`;
- required `host_permissions` remain forbidden;
- `XMLHttpRequest`, WebSocket, EventSource, sendBeacon and remote script/resource URLs remain forbidden;
- built JavaScript that uses network fetch must contain the fixed IPv4 loopback endpoint and no non-loopback absolute HTTP(S) URL.

## Consequences

Users who never use the companion retain the existing extension-only model and do not grant localhost access. Users who enable companion mode must approve one narrowly scoped runtime host permission and pair each extension-page session with the currently running companion.

M21 may improve companion UX, but it must not persist the M19 session token or move password-based cryptography into the native companion.
