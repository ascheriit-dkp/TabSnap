# ADR 0020 — Versioned machine snapshot container

## Status

Accepted for Level 4 M31.

## Context

M30 can coordinate capture across explicitly connected Chrome, Edge and Firefox instances while keeping every browser workspace encrypted inside its extension. Those encrypted results are ephemeral and disappear when the companion exits.

M31 needs one durable machine snapshot without moving the trust boundary. The Windows companion may package, validate and store encrypted browser results, but it must not receive snapshot passwords, encryption keys or decrypted browser state.

The existing `.tabsnap` file remains the single-browser encrypted snapshot format. A machine snapshot therefore needs a distinct container so old single-browser readers never mistake a multi-browser bundle for one encrypted browser payload.

## Decision

TabSnap adds a versioned binary machine container with the `.tabsnap-machine` extension.

Version 1 starts with:

1. the fixed magic `TABSNAPM\0`;
2. one container-version byte;
3. a big-endian 32-bit manifest length;
4. the manifest;
5. the successful browser payloads concatenated in manifest order.

The manifest contains only bounded coordination metadata:

- capture job identifier;
- persistence timestamp;
- target browser instance identifier;
- browser kind and optional bounded browser version;
- either the encrypted payload length for a successful target or one bounded M30 failure code.

It does not contain URLs, titles, plaintext workspace data, passwords or keys.

Each successful payload is the existing encrypted TabSnap envelope returned by the browser extension. The companion treats those bytes as opaque and never decrypts or re-encrypts them.

Machine containers are limited to 32 targets, 16 KiB of manifest data, 65 MiB per encrypted browser payload and 256 MiB of encrypted payload data per container. A persisted container must contain at least one successful encrypted payload; a job where every target failed is reported but is not written as an empty machine snapshot.

Reading is strict. The companion rejects unknown container versions, invalid magic, malformed identifiers, duplicate instance identifiers, unsupported browser metadata, invalid failure codes, truncated manifests, inconsistent payload lengths, trailing bytes, symbolic links and files above the configured limits before allocating a payload-sized buffer.

Persistence is atomic within the selected snapshot directory: write a unique temporary file, flush it with `sync_all`, close it, then rename it to the collision-safe final name. Temporary files are removed after failed writes.

Machine snapshot names use Windows-safe sanitization, including reserved device names such as `CON`, `PRN`, `AUX`, `NUL`, `COM1` through `COM9` and `LPT1` through `LPT9`.

M31 also exposes strict inspection and targeted encrypted-payload reads. Those primitives are intentionally opaque and are the input boundary for M32 coordinated restore.

## Consequences

- A completed M30 job can survive companion shutdown as one portable machine snapshot.
- Single-browser `.tabsnap` compatibility is unchanged.
- The companion can inspect routing metadata without learning browsing history or passwords.
- Partial captures remain explicit: failed targets are recorded next to successful encrypted targets.
- M32 can select and forward one encrypted browser payload without parsing its plaintext.
- The machine manifest is metadata, not an authenticated encryption envelope; integrity of each browser workspace remains enforced by the encrypted TabSnap payload when the destination extension decrypts it.
