# Chrome extension security audit

Level 1 has a machine-enforced release invariant: the built Chrome extension must remain local-only and minimally privileged.

Run:

```bash
pnpm extension:build
pnpm extension:audit
```

`pnpm check` runs both automatically in CI.

## Permission invariant

The built `manifest.json` must:

- use Manifest V3
- request exactly `tabs` and `tabGroups`
- have no host permissions
- have no optional host permissions
- have no optional permissions
- have no content scripts
- have no externally-connectable endpoint

`tabs` is used to capture privileged tab metadata such as URLs and titles. `tabGroups` is used to capture and restore group metadata.

Any permission expansion is a security-significant product change and must update the threat model and this audit document.

## Persistent workspace invariant

The toolbar action must not use `action.default_popup`.

Level 1 beta uses the packaged `background.js` service worker only to open or focus the persistent local `index.html` workspace. This prevents long capture, Argon2id, decryption and restore operations from depending on the lifetime of an ephemeral popup.

The launcher receives no extra permission and is covered by the same final-artifact network audit as the rest of the extension JavaScript.

## CSP invariant

Extension pages must load scripts from the packaged extension only.

`wasm-unsafe-eval` is allowed solely because the packaged Argon2id implementation uses WebAssembly. Remote HTTP(S) script/network sources are not permitted by the manifest audit.

## Built-bundle network audit

The audit recursively examines executable/text assets in `apps/chrome-extension/dist` after Vite has bundled workspace code and dependencies.

It rejects:

- absolute `http://` or `https://` resource references in HTML, CSS and JSON assets
- `fetch()` in executable JavaScript
- `XMLHttpRequest`
- `WebSocket`
- `EventSource`
- `sendBeacon`
- `importScripts()`

JavaScript dependencies may contain inert URL strings that are never requested. For example, Zod includes JSON Schema identifier URIs and constructs a synthetic `http://[IPv6]` URL locally when validating IPv6 syntax. The audit therefore targets network-capable JavaScript APIs rather than rejecting every URL-looking string inside bundled code.

Source maps are excluded because they are non-executable debug metadata and can legitimately contain upstream source comments or URLs.

This check deliberately runs on the final artifact rather than only grepping source files: a dependency or bundler transform that introduced a network-capable path would still be caught by these invariants.

## Limits of the audit

Static matching is defense in depth, not a proof of non-communication. JavaScript can synthesize identifiers and URLs dynamically or use an API the matcher does not yet know about.

For that reason Level 1 also relies on:

- no host permissions
- no content scripts
- a restrictive extension CSP
- browser integration tests
- release review of permission and dependency changes

## Crypto-import resource audit

Envelope v1 accepts one fixed Argon2id profile:

- memory: 65,536 KiB
- iterations: 3
- parallelism: 4
- output: 32 bytes

These values are parsed before Argon2 runs. A malicious unauthenticated header therefore cannot raise the KDF cost or select a weaker profile while still claiming to be a v1 TabSnap envelope.

A future KDF profile change requires a deliberate format/version decision rather than silently widening v1 parser bounds.
