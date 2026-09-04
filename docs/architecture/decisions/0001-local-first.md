# ADR 0001: Local-first snapshots

Status: accepted

## Decision

TabSnap does not store user snapshots on a TabSnap server.

The user owns and moves the encrypted payload.

## Consequences

- no account system
- no snapshot database
- no sync backend
- offline operation is a requirement
- password recovery is impossible by design
