# Architecture

The core does not care which browser produced a snapshot.

```text
browser adapter
     |
     v
universal snapshot
     |
     v
serialize -> compress -> encrypt
     |
     v
string / .tabsnap
```

Restore runs the other way around.

Browser-specific code stays behind adapters.

Architecture decisions live in `docs/architecture/decisions/`.
