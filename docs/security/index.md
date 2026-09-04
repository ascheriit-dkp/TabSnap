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

Losing the password means losing access to the encrypted snapshot. There is no recovery service.

The detailed threat model lands with the crypto layer.
