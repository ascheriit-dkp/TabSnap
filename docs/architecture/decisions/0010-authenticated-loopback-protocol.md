# ADR 0010 — Authenticated loopback companion protocol

## Status

Accepted for M19.

## Context

The Chrome extension needs a future way to exchange already-encrypted snapshots with the optional portable companion. The transport must preserve TabSnap's local-only model, work without an installer or administrator privileges, and avoid turning the companion into a LAN or Internet service.

A localhost listener without authentication is not sufficient: other local processes and browser contexts can reach loopback, and web pages can attempt browser requests to private-network endpoints.

## Decision

The companion exposes a deliberately small versioned HTTP/1.1 protocol on an operating-system-selected port bound only to IPv4 loopback (`127.0.0.1`). There is no configuration or command-line option for wildcard, LAN or Internet-facing binding.

Each server process generates a fresh 256-bit session token from the operating-system cryptographic random source. Windows uses `BCryptGenRandom`. The token is held only in memory and printed as part of an explicit pairing code:

```text
tabsnap-companion:v1:<port>:<token>
```

The token is not written to the companion config, snapshot library or registry. Restarting the process creates a new port/token session.

All protocol operations require `Authorization: Bearer <token>` except CORS `OPTIONS` preflight, which performs no operation.

If an HTTP `Origin` header is present, only a syntactically valid `chrome-extension://<32-character-id>` origin is accepted. Normal web origins are rejected independently of Bearer authentication. Allowed extension origins receive narrowly scoped CORS response headers.

Protocol v1 contains only:

- `GET /v1/status`
- `GET /v1/snapshots`
- `POST /v1/snapshot`
- `GET /v1/snapshot`

The protocol transfers opaque encrypted snapshot bytes and metadata limited to file name and size. It does not decrypt, decompress or parse browser workspace contents and does not handle passwords.

The HTTP parser is intentionally narrow and implemented with the Rust standard library. It accepts HTTP/1.1 only, GET/POST/OPTIONS only, fixed `Content-Length` bodies only, rejects duplicate headers and `Transfer-Encoding`, caps headers at 16 KiB and bodies at 65 MiB, and uses finite socket read/write timeouts.

## Why not TLS on loopback

The transport never leaves the local machine and is bound only to loopback. Confidentiality of the snapshot payload is already provided by the TabSnap encrypted envelope, while authorization is provided by a fresh high-entropy session token. Adding a self-signed TLS identity would introduce certificate provisioning and trust UX without protecting against a local process that already possesses the pairing token.

The relevant boundary is therefore authenticated local access, not remote network confidentiality.

## Consequences

The companion gains a network listener for the first time, but the listener cannot accept LAN or Internet connections.

The explicit pairing code creates an opt-in handoff between the companion process and the Chrome bridge planned for M20. A stale pairing code stops working when the companion restarts.

A malicious ordinary website is blocked both by the absence of the session token and by the browser-origin policy. A malicious local process with access to the live pairing token is inside the local-user threat boundary and can use the protocol for that session.

Future protocol changes require a new compatible version decision rather than silently widening v1.
