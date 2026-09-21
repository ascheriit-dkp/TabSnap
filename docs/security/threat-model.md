# Threat model

TabSnap is a local-first browser workspace migration tool for Chrome, Microsoft Edge and Firefox, with an optional portable Windows companion.

The protected data is browser workspace metadata: normal windows, tab URLs/titles/order/pinning, tab-group metadata and browser/platform metadata. A single-browser export is an encrypted `.tabsnap` envelope. A whole-machine export is a `.tabsnap-machine` container holding bounded routing metadata plus one existing encrypted TabSnap payload per successful browser capture.

## Security goals

TabSnap aims to provide:

- confidentiality of exported browser workspace data when the password is not known;
- integrity and authenticity of each encrypted TabSnap envelope, including its versioned header;
- bounded handling of malformed or intentionally hostile encrypted snapshots, machine containers and localhost protocol requests;
- local encryption/decryption inside browser extensions, not inside the companion;
- explicit pairing before any browser participates in whole-machine coordination;
- IPv4-loopback-only companion transport;
- bounded, ephemeral capture/restore job state with no plaintext browsing data;
- non-destructive browser restore by default;
- no TabSnap account, backend, telemetry, analytics or cloud sync;
- no stored snapshot password or derived encryption key.

TabSnap does not claim to protect data on an already-compromised endpoint.

## Trust boundaries

### Browser extension APIs

Chrome, Edge and Firefox adapters trust their browser APIs to return normal windows/tabs/groups and to enforce browser restrictions during restore.

Extension permissions remain limited to browser workspace APIs such as `tabs` and `tabGroups`. Companion access to `http://127.0.0.1/*` is optional and requested only for explicit companion use.

There are no content scripts and no externally-connectable page endpoint.

### Extension-page memory

Plaintext snapshots, snapshot passwords and derived keys exist transiently in the extension page while capture/encryption or decryption/restore is running.

They are not written to extension storage and are never sent to the Windows companion. JavaScript runtimes do not provide a guarantee of immediate physical memory erasure after objects become unreachable.

### Clipboard and user-selected transport

An encrypted `tabsnap:v1:` string may be copied to the clipboard. Clipboard contents and any third-party service chosen by the user to move an encrypted file/string are outside TabSnap's control.

The password is never copied automatically.

### Browser snapshot files

A `.tabsnap` file contains an authenticated encrypted envelope. Imported files are attacker-controlled until strict envelope parsing, authentication, bounded decompression and snapshot schema validation succeed.

### Whole-machine container

A `.tabsnap-machine` file contains:

- a versioned bounded manifest;
- capture job metadata;
- browser kind and bounded version metadata;
- ephemeral browser instance identifiers;
- bounded capture failure codes;
- concatenated encrypted `.tabsnap` payloads.

The machine manifest is routing metadata, not an encryption envelope. It must not contain URLs, titles, passwords, keys or decrypted workspace state.

The companion validates container magic/version, manifest length, target count, instance-id uniqueness, browser/version metadata, failure codes, per-browser payload lengths, total payload size, exact file length, regular-file status and extension before routing a payload.

Each browser payload still relies on the authenticated TabSnap envelope for confidentiality/integrity of browser state.

### Windows companion process

The companion stores and routes opaque encrypted payloads. It may know local file names/paths and bounded browser coordination metadata, but it must not receive snapshot passwords, keys or plaintext workspace contents.

The native UI starts with the protocol stopped. The user explicitly starts the loopback listener and pairs browser pages. The UI does not discover browser processes, enumerate profiles or contact a cloud service.

### Local loopback protocol

The companion binds only to IPv4 loopback (`127.0.0.1`) on an OS-selected port.

Each process creates a fresh 256-bit session token from the OS random source. The token is included in the explicit pairing code, used as a Bearer credential and is not persisted.

Browser coordination endpoints additionally require a strict extension origin:

- Chromium: `chrome-extension://<32-char extension id>`;
- Firefox: `moz-extension://<uuid>`.

Browser instance identifiers are ephemeral, origin-bound and leased. A different extension origin cannot take over an existing instance identifier.

HTTP parsing is intentionally small and bounded: HTTP/1.1 only, bounded headers/bodies, no `Transfer-Encoding`, no duplicate headers, POST requires `Content-Length`, and bytes beyond the declared body are rejected.

## Attackers considered

### Encrypted snapshot thief

An attacker obtains an encrypted `.tabsnap` payload, encrypted string or a `.tabsnap-machine` bundle but not the password.

Mitigation: Argon2id derives a 256-bit key from the password; AES-256-GCM protects each compressed browser snapshot.

Residual risk: weak or reused passwords can be guessed offline. Argon2id increases the cost but cannot make a weak password strong.

### Encrypted snapshot modifier

An attacker changes ciphertext, IV, salt, version, KDF metadata or other authenticated envelope fields.

Mitigation: AES-GCM authenticates the complete serialized envelope header as additional authenticated data. Modified authenticated data or ciphertext does not decrypt.

Envelope v1 accepts only TabSnap's fixed KDF profile: 64 MiB, 3 iterations, parallelism 4. Imported metadata cannot request an attacker-chosen pre-authentication Argon2 cost.

### Hostile machine-container attacker

An attacker supplies a truncated, oversized or structurally manipulated `.tabsnap-machine` file.

Mitigations include:

- fixed magic and container version;
- 16 KiB manifest limit;
- maximum 32 browser targets;
- unique 128-bit instance identifiers;
- strict browser/version/failure-code parsing;
- 65 MiB maximum per encrypted browser payload;
- 256 MiB maximum encrypted payload total;
- exact payload/file-length agreement;
- at least one successful encrypted payload;
- regular-file requirement and symbolic-link rejection;
- payload allocation only after strict metadata and size validation.

The companion does not parse decrypted browser JSON from a machine container.

### Malformed browser-snapshot attacker

An attacker provides invalid UTF-8, JSON, schema values, Base64URL, gzip data, unsupported envelope versions or inconsistent browser state.

Mitigations include:

- strict runtime schema validation with unknown fields rejected;
- bounded encrypted, header and decompressed sizes;
- fixed v1 KDF parameters;
- AES-GCM authentication before decompression/restore;
- URL policy validation before browser API calls;
- per-tab/group restore isolation and bounded warnings.

Residual risk: parsers, decompressors, WebAssembly runtimes, browser APIs and JavaScript engines can contain implementation bugs. Limits reduce exposure but do not eliminate it.

### Malicious webpage

A normal webpage should not be able to read TabSnap data or operate the companion merely because TabSnap is installed.

Mitigations: no content scripts, no externally-connectable extension endpoint, strict loopback CORS extension-origin validation and Bearer authentication.

### Unpaired local process

Another local process can connect to `127.0.0.1` but should not be able to use the companion API without the process-scoped 256-bit token.

Requests without the token are rejected. Browser coordination also requires a valid paired extension origin and an origin-bound ephemeral browser instance.

Residual risk: malware able to read another process's memory/clipboard/keystrokes or otherwise steal the current pairing token is already inside the endpoint trust boundary.

### Malicious or compromised extension

A different browser extension cannot claim another origin-bound instance identifier, but an extension or browser that is already compromised can access its own browser data and may observe secrets before TabSnap encrypts or after it decrypts them.

This is out of scope for TabSnap's exported-data encryption guarantee.

### Resource-exhaustion attacker

Hostile files or loopback requests may attempt to force large allocations, fill in-memory job stores or repeatedly claim work.

Mitigations include:

- fixed request/header/body limits;
- bounded registry, capture-job and restore-job counts;
- bounded per-result and per-job encrypted bytes;
- claim leases;
- target timeouts and job retention;
- terminal-state rejection of duplicate submissions;
- strict machine-container length checks before payload allocation.

M34 regression tests exercise hostile manifests, protocol limits, inter-job isolation and restore-origin binding.

### Race/replay attacker

A browser page may retry after a lost response or a job may temporarily lose its claimant.

Capture and restore assignments use leases. Completed targets reject duplicate terminal submissions. Explicit restore retry remaps only failed or browser-unavailable targets; already completed restores are not replayed.

## Restore safety

Restore is non-destructive: TabSnap creates new browser windows and does not close the current workspace.

Imported URLs are data, not instructions. Script/data execution schemes and browser-specific privileged URLs that cannot be safely recreated are rejected or skipped according to the adapter policy. Cross-browser restore is best effort and reports known losses/warnings.

For whole-machine restore, the companion sends one selected encrypted payload to one destination extension. Password verification, authenticated decryption and actual browser restore occur only inside that destination extension.

## Out of scope / not protected

TabSnap does not protect against:

- malware or another extension that can read browser state before encryption or after decryption;
- an OS-level attacker that can inspect process memory, keystrokes or the active pairing token;
- a malicious or compromised browser installation;
- weak snapshot passwords or offline guessing;
- loss of the password; there is no recovery service or escrow;
- secrets intentionally present in captured tab URLs or titles before encryption;
- clipboard compromise;
- filesystem deletion/corruption of encrypted snapshots by a local attacker;
- browser/store policies or privileged pages the browser refuses to restore;
- availability attacks by a process that repeatedly consumes local CPU/disk within OS permissions.

## Release expectations

Before a Level 4 prerelease:

- the full static/unit test suite must pass;
- Windows rustfmt, Clippy, companion tests, release build and executable smoke test must pass;
- Chromium real-extension Playwright integration must pass;
- Edge packaging/audit must pass;
- Firefox build/audit/`web-ext lint`/package must pass;
- hostile container/protocol and multi-job regression tests must pass;
- built extension audits must continue to reject unexpected permissions, remote code and unreviewed network primitives;
- release archives and SHA-256 checksum assets must be produced by the release workflow.

Before declaring `1.0.0` stable, TabSnap also requires the manual multi-browser/store/portable QA listed in the Level 4 readiness review. Automated CI passing alone does not promote the project to 1.0.
