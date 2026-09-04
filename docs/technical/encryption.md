# Encryption

Snapshots are compressed before encryption.

Current envelope version: `1`.

## KDF

Argon2id v1.3:

- 64 MiB memory
- 3 iterations
- parallelism 4
- 16-byte random salt
- 32-byte output

Passwords shorter than 8 characters are rejected.

## Cipher

AES-256-GCM through Web Crypto:

- 12-byte random IV
- 128-bit authentication tag
- the envelope header is authenticated as additional data

Changing the ciphertext or authenticated header makes decryption fail.

## Envelope

The binary layout is:

```text
8 bytes   magic: TABSNAP\\0
1 byte    envelope version
4 bytes   header length, big endian
N bytes   UTF-8 JSON header
rest      AES-GCM ciphertext + tag
```

The header contains the KDF parameters, salt, cipher, IV and compression algorithm.

A copyable string is the same binary envelope encoded as Base64URL with this prefix:

```text
tabsnap:v1:
```

A `.tabsnap` file will contain the binary envelope directly.

## Runtime

AES-GCM uses Web Crypto.

Argon2id uses packaged local WebAssembly. Chrome Manifest V3 requires `wasm-unsafe-eval` in the extension CSP for that WASM. No remote code is loaded.

There is no password recovery. There is nothing on a server to recover.
