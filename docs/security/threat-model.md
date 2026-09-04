# Threat model

TabSnap Level 1 is a local-only Chrome extension. The protected artifact is an encrypted snapshot containing browser workspace metadata: window geometry/state, tab URLs/titles/order/pinning, and tab-group metadata.

## Security goals

TabSnap aims to provide:

- confidentiality of a snapshot after export, assuming the password is not known
- integrity and authenticity of the encrypted envelope, including its versioned header
- bounded handling of malformed or intentionally hostile imports
- non-destructive restore by default
- no TabSnap-initiated network communication in Level 1
- no stored password or derived encryption key

TabSnap does not claim to protect data on an already-compromised endpoint.

## Trust boundaries

### Chrome browser APIs

The extension trusts Chrome to return the user's normal windows, tabs and tab-group metadata and to enforce browser restrictions during restore.

The extension requests only `tabs` and `tabGroups`. It has no host permissions and no content scripts.

### Popup memory

Plaintext snapshots, passwords and derived keys exist transiently in extension-page memory while an operation is running. They are not written to extension storage.

Closing the popup ends that UI context, but TabSnap does not claim guaranteed immediate memory erasure by the JavaScript engine.

### Clipboard

A copyable encrypted string may be placed on the clipboard. Clipboard contents are outside TabSnap's control after the write completes.

The password is never copied automatically.

### Filesystem

A `.tabsnap` file is encrypted before download. Imported files are attacker-controlled until parsing, validation and authenticated decryption succeed.

## Attackers considered

### Snapshot thief

An attacker obtains an encrypted `.tabsnap` file or `tabsnap:v1:` string but not the password.

Mitigation: Argon2id derives a 256-bit key from the password; AES-256-GCM protects the compressed snapshot.

Residual risk: weak or reused passwords can be guessed offline. Argon2id raises the cost but cannot make a weak password strong.

### Snapshot modifier

An attacker changes ciphertext, IV, salt, version, KDF metadata or other authenticated header fields.

Mitigation: AES-GCM authenticates the complete serialized header as additional authenticated data. Modified authenticated data or ciphertext does not decrypt.

For envelope v1, KDF parameters are accepted only at TabSnap's fixed profile (64 MiB, 3 iterations, parallelism 4). An attacker therefore cannot force an imported v1 envelope to run an arbitrarily expensive Argon2 configuration before authentication.

### Malformed-import attacker

An attacker provides invalid UTF-8, JSON, schema values, oversized payloads, invalid Base64URL, malformed gzip data, unsupported envelope versions or deliberately inconsistent browser state.

Mitigations include:

- strict runtime schema validation with unknown fields rejected
- bounded encrypted, header and decompressed sizes
- fixed v1 KDF parameters
- AES-GCM authentication before snapshot decompression/restore
- per-tab restore isolation so one rejected URL does not abort the full restore
- explicit rejection of `javascript:`, `data:`, `blob:`, `filesystem:`, `devtools:` and `chrome-extension:` restore URLs

Residual risk: any parser, decompressor, WebAssembly runtime, browser API or JavaScript engine can contain implementation bugs. Size and schema limits reduce exposure but do not eliminate it.

### Malicious webpage

A webpage should not be able to communicate with TabSnap or receive snapshot data merely because the extension is installed.

Mitigation: there are no content scripts, host permissions or externally-connectable extension endpoints.

### Network observer or remote server

Level 1 has no application network protocol and no backend.

Mitigation: CI audits the built extension for host permissions, remote URLs and common network APIs such as `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource` and `sendBeacon`.

This is a defense-in-depth static audit, not a mathematical proof that a JavaScript program can never communicate. Browser integration testing and manual release review remain required.

## Out of scope / not protected

TabSnap does not protect against:

- malware or another extension that can read browser state before encryption or after decryption
- an OS-level attacker that can inspect process memory or keystrokes
- clipboard malware reading an encrypted snapshot string or a password the user copied manually
- a malicious or compromised Chrome installation
- loss of the password; there is no recovery service or escrow
- secrets already present inside tab URLs or titles; those are intentionally part of the snapshot
- browser-specific pages that Chrome refuses to recreate; these are skipped and reported

## Restore safety

Restore is non-destructive: it creates new windows and does not close the user's current workspace.

Imported URLs are data, not script instructions. Schemes with direct script/data execution semantics are rejected before Chrome APIs are called. Other unusual URLs may still be rejected by Chrome and are reported as warnings.

## Release expectations

Before a Level 1 alpha/beta release:

- the automated extension security audit must pass on the built artifact
- unit and browser integration tests must pass
- permission changes require explicit review and documentation
- any newly introduced network primitive or host permission is treated as a security-significant change
