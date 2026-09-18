# ADR 0021 — Coordinated restore routing

## Status

Accepted for Level 4 M32.

## Context

M31 stores one machine snapshot as bounded routing metadata plus the existing encrypted TabSnap payload for each browser capture that succeeded.

M32 must restore those payloads into currently connected Chrome, Edge and Firefox instances without moving the cryptographic trust boundary into the Windows companion. Browser availability may differ from capture time, a captured source may no longer have a matching live instance, and one restore attempt may succeed only partially.

## Decision

The companion owns restore routing and bounded job state, but never decrypts a browser payload.

A restore job is created from one strictly validated `.tabsnap-machine` file and the current ephemeral browser registry. Only instances advertising the `restore` capability are eligible.

For every successful source payload, destination selection uses three deterministic passes:

1. reuse the same live browser instance identifier when it is still connected;
2. otherwise use an unused live instance of the same browser kind;
3. otherwise use an unused restore-capable browser instance as a cross-browser fallback.

Native matches are reserved before cross-browser fallback so an earlier source cannot consume a destination required by a later native source. A destination instance receives at most one source payload in one restore job.

A machine target whose capture failed has no encrypted payload and is marked `source-unavailable`. It is not retryable. A successful source with no current destination is marked `browser-unavailable` and can be retried after another browser connects.

Restore assignments use the existing authenticated IPv4-loopback session and extension-origin binding. The companion returns only the targeted encrypted browser payload plus bounded routing headers. It never sends another target's payload to that extension.

The destination extension:

- validates the restore assignment and payload size;
- requires its local snapshot password;
- decrypts the existing TabSnap envelope locally;
- runs the existing browser adapter restore path;
- therefore preserves the existing Chrome, Edge and Firefox cross-browser URL and feature-loss rules;
- sends only a bounded success acknowledgement or one of `password-required`, `decrypt-failed` or `restore-failed` back to the companion.

Passwords, encryption keys, decrypted snapshots, URLs, titles and raw browser exception text never enter restore job state or the companion protocol.

Restore jobs are in memory only. Claims use a short lease so interrupted polling can be retried. Unfinished assignments time out, completed targets are never replayed by an explicit retry, and retry remaps only failed or `browser-unavailable` targets against the then-current registry.

The companion CLI exposes a diagnostic `restore <file.tabsnap-machine>` flow for M32 validation. The final whole-machine user interface remains M33.

## Consequences

- A machine snapshot can be restored even when the exact original browser processes are gone.
- Same-browser restoration is preferred without blocking deliberate cross-browser fallback.
- Partial success, skip and retry states are explicit per source payload.
- A successful restore is not replayed when another target is retried.
- The companion remains an opaque encrypted-payload router rather than a workspace decryption endpoint.
- Cross-browser fidelity continues to be decided by the existing extension restore adapters after authenticated decryption.
