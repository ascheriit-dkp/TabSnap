# Security

TabSnap handles browser workspace data, including URLs and tab titles. Those values can contain private information.

## Baseline

- no TabSnap backend
- no telemetry or analytics
- no password or derived-key storage
- authenticated encryption for browser snapshots
- encryption/decryption stays inside browser extensions
- imported snapshots and machine containers are treated as hostile input
- optional companion binds only to `127.0.0.1`
- fresh 256-bit in-memory pairing token per companion process
- explicit Chrome, Edge and Firefox pairing; no browser-process discovery
- bounded registry/job/body/payload sizes
- leases, timeouts and duplicate-terminal rejection for coordinated work
- whole-machine restore does not replay completed targets
- CI audits built extension permissions, network primitives and remote-code policy

A `.tabsnap-machine` manifest contains bounded routing metadata around already-encrypted browser payloads. It is not a second encryption layer and must not contain browser plaintext.

Losing the snapshot password means losing access to the encrypted browser payload. There is no recovery service.

Read the [threat model](./threat-model), [Level 4 / 1.0 readiness review](./level4-readiness) and [extension security audit](./extension-audit).
