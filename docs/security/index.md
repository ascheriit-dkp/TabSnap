# Security

TabSnap deals with URLs. URLs can contain private data.

## Baseline

- no backend
- no telemetry
- no analytics
- no remote dependency required at runtime
- no password storage
- authenticated encryption
- imported snapshots are hostile input
- Chrome Level 1 requests no host permissions
- CI audits the final extension bundle for network primitives and permission expansion

Losing the password means losing access to the encrypted snapshot. There is no recovery service.

Read the [threat model](./threat-model) and the [Chrome extension security audit](./extension-audit).
