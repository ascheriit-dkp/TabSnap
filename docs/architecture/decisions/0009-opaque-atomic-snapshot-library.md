# ADR 0009 — Opaque atomic snapshot library

## Status

Accepted for M18.

## Context

The Windows companion needs to organize encrypted `.tabsnap` files before browser IPC exists. Pulling snapshot decryption into the companion at this stage would duplicate the Level 1 cryptographic surface, require password handling in a second process and weaken the simple local-only boundary.

File operations also need to be safe on removable media and must not silently overwrite an existing snapshot when two imports share a name.

## Decision

The M18 library treats every `.tabsnap` payload as opaque encrypted bytes.

It may:

- discover regular `.tabsnap` files in the active library directory
- import an existing encrypted file
- export a named library file
- atomically store an already-encrypted byte sequence for future IPC

It does not decrypt, decompress or parse snapshot payloads and does not handle passwords.

Snapshot files are limited to 65 MiB. Symbolic links and non-regular files are not accepted as snapshot sources.

Every write or copy is staged in a uniquely created temporary file in the same destination directory, flushed, then renamed to the final `.tabsnap` path. Final names are chosen without overwriting existing files.

User-controlled names are normalized for Windows-invalid characters and reserved DOS device names. Library export accepts only one file-name component, preventing directory traversal.

## Consequences

The companion can provide a useful portable snapshot library while preserving the existing encryption boundary: encrypted bytes remain encrypted at rest and the companion cannot inspect browser URLs or tabs.

A crash can leave a non-`.tabsnap` temporary file, but discovery will not present it as a valid snapshot. Successful and failed operations attempt to remove their temporary files.

M19 and M20 can reuse the internal atomic write path when authenticated browser IPC begins transferring encrypted envelopes.
