# ADR 0018 — Ephemeral browser coordination

## Status

Accepted for Level 4 M29.

## Context

Level 4 needs one Windows companion to coordinate TabSnap extensions running in Chrome, Edge and Firefox on the same machine. The existing companion protocol can store and return opaque encrypted `.tabsnap` files, but it has no concept of which browser instances are currently connected.

The companion must not become a new source of browsing-history data. It also must not weaken the existing rule that snapshot passwords and decrypted workspaces stay inside extensions.

## Decision

The companion keeps an **ephemeral in-memory browser registry** behind the existing authenticated IPv4-loopback protocol.

A connected extension registers:

- a random 128-bit instance identifier generated for that page session;
- browser kind: `chrome`, `edge` or `firefox`;
- a bounded browser version when available;
- bounded coordination capabilities such as `capture` and `restore`.

Registry entries are bound to the extension origin that registered them. A different extension origin cannot refresh or replace the same instance identifier.

Entries use a 30-second lease. Extensions heartbeat periodically while companion mode remains connected. Stale entries are removed automatically. The registry is capped at 32 live instances.

The registry is never written to disk. It does not contain tabs, URLs, window titles, snapshot plaintext, encryption keys or passwords.

Coordination is added to protocol v1 through additive endpoints and additive status fields. Existing snapshot-library operations and pairing codes remain compatible. An older companion can still be used for the Level 2 snapshot library, but it will not advertise Level 4 coordination support.

## Consequences

- M30 can address explicit live browser instances without browser-process discovery or Internet access.
- Closing an extension page or losing the companion naturally removes the instance after the lease expires.
- The companion learns only bounded runtime/capability metadata for extensions that the user explicitly connected.
- Future capture/restore orchestration still needs a separate design that preserves end-to-end encryption; M29 does not give the companion decrypted browser state.
