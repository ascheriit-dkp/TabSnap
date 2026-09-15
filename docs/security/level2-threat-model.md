# Level 2 threat model

Level 2 adds an optional portable Windows companion and an opt-in Chrome bridge. The Level 1 encrypted snapshot format and browser-side cryptography remain unchanged.

## Added trust boundaries

### Portable companion process

The companion manages encrypted `.tabsnap` files, storage paths and an ephemeral local protocol token. It does not receive snapshot passwords and does not decrypt, decompress or interpret browser-session contents.

A compromise of the companion process can read or replace encrypted snapshot files and pairing tokens available to that process. It does not by itself reveal snapshot plaintext without also compromising the browser/password boundary or breaking the snapshot encryption.

### Local filesystem

Portable mode stores encrypted snapshots beside `tabsnap-companion.exe`. Local and custom modes store them only in the explicitly selected directory.

Risks include removable-media loss, local malware modifying files, malicious files placed into the library and storage paths becoming unavailable or read-only.

Mitigations:

- encrypted snapshots remain opaque at rest;
- discovery accepts only immediate regular `.tabsnap` files;
- symbolic links, directories and oversized files are ignored/rejected;
- imports and protocol writes are size-bounded;
- writes stage into same-directory temporary files, flush, then rename;
- existing snapshots are never silently overwritten;
- custom storage must be absolute and is write-tested before activation.

### Loopback protocol

The companion can listen only on IPv4 loopback (`127.0.0.1`) using an operating-system-selected port. There is no LAN or wildcard bind option.

Each server process generates a fresh 256-bit session token from the operating-system cryptographic RNG. Real operations require the token as Bearer authentication. The token is not persisted. Restarting the process invalidates it.

The HTTP surface is deliberately small and rejects malformed requests, duplicate headers, `Transfer-Encoding`, unsupported methods and oversized bodies. Socket operations have finite timeouts and responses use `Cache-Control: no-store`.

Residual risk: another process running as the same user can attempt to connect to loopback. Authentication makes knowledge of the ephemeral pairing token the authorization boundary. An attacker that can inspect the companion or extension process memory can obtain that token and is already inside the endpoint-compromise model.

### Browser permission boundary

The Chrome extension declares `http://127.0.0.1/*` only as an optional host permission. It is requested from an explicit Connect gesture after the user provides a pairing code.

Without that opt-in, Level 1 operation remains unchanged and the extension does not contact localhost.

The pairing parser accepts only the versioned `tabsnap-companion:v1:<port>:<token>` form and constructs only an IPv4 loopback endpoint. Redirects and browser credentials are disabled. The bridge validates the protocol version, transport and authentication mode before treating a companion as connected.

Normal web origins are not accepted by the companion CORS policy. The protocol allows Chrome-extension origins only; preflight performs no library operation and subsequent requests still require Bearer authentication.

### Native companion UI

The M21 UI is a Win32 surface inside the same portable process. It exposes existing library, storage and protocol operations. It does not embed a WebView or execute remote content.

The listener is stopped when the UI opens and starts only after the user presses Start protocol. The pairing code is displayed/copyable only for the current process lifetime.

Clipboard contents and file-picker selections leave TabSnap's control once handed to Windows or another application.

## Attackers considered

### Lost USB drive

An attacker obtains the portable companion directory and encrypted snapshots.

Mitigation: snapshot plaintext remains protected by the Level 1 password-derived encryption. The companion config contains storage settings only and no password, key or persistent pairing token.

Residual risk: snapshot file names may reveal user-chosen labels. Weak snapshot passwords remain susceptible to offline guessing.

### Malicious local process or webpage

A webpage should not gain snapshot access because the companion is running.

Mitigations include loopback-only binding, a high-entropy Bearer token, Chrome-extension-only CORS responses and the absence of externally-connectable extension messaging.

A malicious local process that already has same-user memory inspection or code-execution capability is outside the protection boundary.

### Malicious `.tabsnap` file

The companion treats snapshot contents as opaque bytes and therefore does not expose the crypto/schema/decompression parsers. Decryption and parsing occur only in the extension under the existing Level 1 bounds and authentication checks.

## Security invariants for Level 2

A Level 2 release must preserve these invariants:

- no snapshot password or derived key is sent to or stored by the companion;
- no companion listener binds outside `127.0.0.1`;
- the pairing token is random, process-scoped and non-persistent;
- the Chrome loopback permission remains optional and user-initiated;
- companion storage contains encrypted snapshot bytes only;
- imported/exported bytes remain unchanged unless a new encrypted snapshot is produced by the browser;
- no account, cloud sync, telemetry endpoint, content script or remote UI runtime is required.

## Release checks

Before a Level 2 prerelease:

- Node checks and Chromium integration tests must pass;
- Rust formatting, Clippy and tests must pass on Windows;
- the release-mode companion executable must build on the Windows runner;
- the built executable must pass the portable CLI smoke test in an isolated temporary directory;
- the portable ZIP and its SHA-256 checksum must be produced from the built executable;
- Chrome and companion release versions must match the release tag;
- the GitHub prerelease must attach both platform archives and both checksums.
