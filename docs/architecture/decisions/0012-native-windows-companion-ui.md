# 0012 — Native Windows companion UI

## Status

Accepted for M21.

## Decision

The Level 2 companion gets a small Win32 UI inside the existing portable Rust executable.

The UI is opened explicitly with:

```text
tabsnap-companion ui
```

The existing CLI remains supported.

The UI uses Windows system APIs directly and adds no WebView, browser runtime, installer, service, background startup entry or third-party GUI dependency.

## Scope

The M21 window exposes only existing companion capabilities:

- list encrypted `.tabsnap` files in the active library;
- refresh the library;
- import and export encrypted snapshot files;
- copy the selected snapshot path;
- select portable, local or custom storage and apply it through the existing write-tested storage policy;
- start the authenticated loopback protocol;
- display and copy the ephemeral pairing code;
- display the active storage path, drive hint and privacy state.

The UI never asks for a TabSnap password and never decrypts, decompresses or parses snapshot contents.

## Protocol lifecycle

The local protocol is stopped by default when the UI opens.

`Start protocol` creates the same M19 loopback-only server used by the CLI `serve` command. The pairing token remains process-memory-only. Closing the companion process stops the listener and invalidates the token.

M21 intentionally does not add auto-start, LAN binding, Internet access or token persistence.

## Portability

The GUI is part of the same `tabsnap-companion.exe` artifact built by Windows CI. Portable storage still resolves relative to the executable location, so running the binary from removable media keeps the same M16/M17 behavior.

## Security boundary

The companion remains an opaque encrypted-file manager and authenticated local bridge. The browser extension remains responsible for encryption/decryption and browser-session interpretation.
