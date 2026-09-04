# ADR 0005: Argon2id and AES-256-GCM

Status: accepted

## Decision

Use Argon2id to derive a 256-bit key from the user password.

Use AES-256-GCM through Web Crypto for authenticated encryption.

Authenticate the versioned envelope header as additional data.

## Argon2id v1 parameters

- 64 MiB
- 3 iterations
- parallelism 4
- 16-byte random salt

The parameters are stored in the envelope and bounded during import.

## AES-GCM

- 256-bit key
- 96-bit random IV
- 128-bit authentication tag

## WASM

The Argon2 implementation is packaged with the extension. No remote code.

Manifest V3 needs `wasm-unsafe-eval` to execute packaged WebAssembly.
