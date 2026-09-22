# Architecture

TabSnap keeps the browser workspace format independent from the browser that produced it.

## Single-browser path

```text
Chrome / Edge / Firefox adapter
            |
            v
    universal snapshot
            |
            v
serialize -> compress -> encrypt
            |
            v
 encrypted string / .tabsnap
```

Restore runs the other way around. Browser-specific behavior stays behind adapters.

## Whole-machine path

```text
paired browser extensions
        |       |       |
        | encrypt locally
        v       v       v
 encrypted .tabsnap payloads
        \       |       /
         \      |      /
          v     v     v
       Windows companion
              |
              v
       .tabsnap-machine
              |
       route encrypted payload
              |
              v
      destination extension
              |
       decrypt + restore locally
```

The companion owns ephemeral coordination, bounded job state, local storage and encrypted-payload routing. It does not receive snapshot passwords, encryption keys or decrypted browser state.

Whole-machine restore prefers the same live instance, then the same browser kind, then an unused cross-browser destination.

Architecture decisions live in [`docs/architecture/decisions/`](./decisions/).
