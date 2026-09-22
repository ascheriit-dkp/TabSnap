# Windows companion

The optional Windows companion is a portable native executable. It adds local snapshot storage and whole-machine coordination for explicitly paired Chrome, Microsoft Edge and Firefox pages.

Single-browser extension workflows still work without it.

## Run it

The native UI is the normal entry point:

```text
tabsnap-companion ui
```

The CLI remains available:

```text
tabsnap-companion info
tabsnap-companion init
tabsnap-companion ui
tabsnap-companion serve
tabsnap-companion capture
tabsnap-companion restore <file-name.tabsnap-machine>
tabsnap-companion storage show
tabsnap-companion storage set portable
tabsnap-companion storage set local
tabsnap-companion storage set custom <absolute-path>
tabsnap-companion library list
tabsnap-companion library import <snapshot.tabsnap>
tabsnap-companion library export <file-name.tabsnap> <destination-directory>
```

For the full native UI flow, see [Whole-machine companion UI](./windows-companion-ui).

## Portable layout

The companion derives its portable root from the executable location:

```text
TabSnap/
├── tabsnap-companion.exe
├── tabsnap-companion.conf   # only after an explicit storage-mode change
└── snapshots/
```

Portable mode is the default when no configuration exists. Running from removable storage therefore keeps the default library beside the executable.

The companion does not install a Windows service, require administrator privileges, write machine-wide registry keys or copy itself into `Program Files`.

## Storage modes

### Portable

Stores snapshots in `snapshots/` beside `tabsnap-companion.exe`.

### Local

Stores snapshots in:

```text
%LOCALAPPDATA%\TabSnap\snapshots
```

### Custom

Stores snapshots directly in an explicitly selected absolute directory.

Every selected storage directory is write-tested before activation. A failed test does not replace the current mode.

## Snapshot libraries

The same storage root can contain two kinds of TabSnap files.

### `.tabsnap`

A single encrypted browser workspace.

The companion lists, imports and exports these files as opaque encrypted bytes. It does not decrypt or decompress them.

### `.tabsnap-machine`

A Level 4 whole-machine container.

It contains bounded routing metadata plus the existing encrypted `.tabsnap` payload from each browser capture that succeeded. The companion validates the container structure and routes encrypted payloads, but still cannot decrypt browser contents.

Library writes are atomic and collision-safe. Symbolic links, unrelated files, invalid containers and files above the configured size limits are not exposed as valid snapshots.

## Local protocol

`serve`, `capture`, `restore` and the native UI use the same local protocol.

The server:

- binds only to IPv4 loopback at `127.0.0.1`;
- uses an operating-system-selected port;
- creates a fresh 256-bit in-memory session token for every process;
- exposes the token only through the pairing code;
- requires explicit browser pairing;
- accepts browser coordination only from valid Chrome-extension or Firefox-extension origins;
- keeps browser registrations and capture/restore jobs in memory;
- applies bounded request, registry, job and payload limits;
- rejects malformed HTTP, chunked transfer, duplicate headers and oversized bodies.

Closing the companion invalidates the session token and removes the ephemeral browser registry/job state.

There is no LAN or Internet listener option.

## Connected browsers

After explicit pairing, the companion tracks only bounded ephemeral metadata:

- random browser instance id;
- browser kind: Chrome, Edge or Firefox;
- bounded browser version when available;
- capture/restore capabilities.

Entries expire unless the paired extension page refreshes its lease.

The companion does not scan processes or browser profiles to discover browsers.

## Coordinated capture

`tabsnap-companion capture` provides the diagnostic CLI flow. The native UI exposes the same operation through **Capture machine**.

Each participating extension:

1. captures its own workspace;
2. encrypts it locally using the password entered in that browser page;
3. sends only the encrypted payload to the companion.

The companion records bounded per-browser state and persists one `.tabsnap-machine` file when at least one browser capture succeeds.

Passwords, derived keys, URLs, titles and decrypted workspace state never enter companion capture state.

## Coordinated restore

`tabsnap-companion restore <file-name.tabsnap-machine>` provides the diagnostic CLI flow. The native UI exposes the same operation through **Restore selected**.

For every successful source payload, routing prefers:

1. the same live browser instance;
2. another live instance of the same browser;
3. an unused compatible browser as cross-browser fallback.

The destination extension receives only its selected encrypted payload, decrypts locally and restores through the existing browser adapter.

Failed or unavailable targets can be retried. Already completed targets are not replayed.

See [Cross-browser compatibility](../technical/cross-browser-compatibility) for known feature-loss rules.

## Privacy boundary

The companion may handle:

- encrypted `.tabsnap` bytes;
- encrypted payloads inside `.tabsnap-machine`;
- local file names and paths;
- storage configuration;
- ephemeral pairing/browser/job metadata.

It does not receive snapshot passwords, encryption keys or decrypted browser workspaces.

There is no TabSnap account, cloud backend, telemetry service, analytics service or hidden browser discovery.
