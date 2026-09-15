# Windows companion

Level 2 adds an optional portable Windows companion.

The companion is not required for the Chrome-only Level 1 workflow. It exists to make local snapshot storage and later browser coordination easier without adding a cloud service.

## Portable layout

The companion derives its portable root from the executable location:

```text
TabSnap/
├── tabsnap-companion.exe
├── tabsnap-companion.conf   # created only after an explicit storage-mode change
└── snapshots/
```

Portable mode is the default when no configuration file exists. If the executable lives on a USB drive, the default snapshot library therefore lives on the same drive automatically.

The companion does not install a Windows service, write machine-wide registry keys, copy itself into `Program Files`, or require administrator privileges.

## Commands

```text
tabsnap-companion info
tabsnap-companion init
tabsnap-companion serve
tabsnap-companion storage show
tabsnap-companion storage set portable
tabsnap-companion storage set local
tabsnap-companion storage set custom <absolute-path>
tabsnap-companion library list
tabsnap-companion library import <snapshot.tabsnap>
tabsnap-companion library export <file-name.tabsnap> <destination-directory>
```

`info` shows the executable location, portable root, active storage mode, resolved snapshot directory and Windows drive hint.

`init` creates and write-tests the active snapshot directory without changing the selected mode.

`storage set ...` resolves the requested location, creates it when needed, writes and deletes a small probe file to verify writability, then persists the selection next to the executable. A failed write test does not replace the existing selection.

`serve` starts the authenticated local companion protocol described below. Closing the process stops the listener and invalidates its session token.

## Storage modes

### Portable

`portable` stores snapshots in `snapshots/` next to `tabsnap-companion.exe`. It is the default and requires no config file.

### Local

`local` stores snapshots in:

```text
%LOCALAPPDATA%\TabSnap\snapshots
```

This mode is explicit. TabSnap does not silently move a portable library into the Windows user profile.

### Custom

`custom` stores snapshots directly in an absolute directory selected by the user. Relative paths are rejected so the meaning of a saved configuration cannot change with the process working directory.

## Snapshot library

M18 adds a file library on top of the active storage directory.

`library list` scans only the immediate directory and reports regular `.tabsnap` files. Directories, symbolic links, unrelated files, oversized files and temporary write files are not exposed as snapshots.

`library import` copies an existing `.tabsnap` file into the active library. It never overwrites an existing snapshot: collisions become names such as `work (2).tabsnap`.

`library export` copies one named library snapshot into another directory with the same collision-safe behavior. The library file name must be a single `.tabsnap` file name, so `..`, absolute paths and directory traversal are rejected.

Incoming names are normalized for Windows-invalid characters and reserved device names such as `CON`, `NUL`, `COM1` and `LPT1`.

Library writes are staged into a uniquely named temporary file in the same target directory, flushed with `sync_all()`, then renamed to the final `.tabsnap` name. Temporary files deliberately do not use the `.tabsnap` extension and are ignored by discovery.

A snapshot file is limited to 65 MiB, matching the Level 1 extension import boundary.

### Opaque encrypted bytes

The companion does not decrypt, decompress or parse `.tabsnap` payloads. Import, export and the local protocol move the encrypted bytes unchanged.

Passwords therefore remain solely in the browser workflow. The companion cannot determine the tabs, URLs or other contents of a stored snapshot.

## Local companion protocol

M19 adds a deliberately narrow HTTP/1.1 protocol for future browser-extension integration.

Run:

```text
tabsnap-companion serve
```

The process write-tests the active storage directory, binds an operating-system-selected port on IPv4 loopback only, then prints output similar to:

```text
TabSnap Companion protocol v1
endpoint: http://127.0.0.1:49152
pairing-code: tabsnap-companion:v1:49152:<session-token>
binding: IPv4 loopback only
session: ephemeral; pairing token is not written to disk
```

The actual port changes between runs. There is intentionally no command-line option to bind the server to `0.0.0.0`, a LAN address or an Internet-facing interface.

### Session authentication

Each server process creates a fresh 256-bit random session token from the operating-system cryptographic RNG. On Windows this uses `BCryptGenRandom`.

Every protocol operation except CORS preflight requires:

```text
Authorization: Bearer <session-token>
```

The token exists only in process memory and in the pairing code printed for that run. It is not written to `tabsnap-companion.conf`, the snapshot library or the Windows registry. Restarting the companion invalidates the old token.

The pairing code is intended to be supplied explicitly to the Chrome extension in M20. Possession of an old pairing code is not enough after the companion process restarts.

### Browser-origin boundary

If a request includes an HTTP `Origin`, the companion accepts only syntactically valid Chrome-extension origins of the form:

```text
chrome-extension://<32-character-extension-id>
```

Normal web origins such as `https://example.com` are rejected even if they somehow present the session token.

For an allowed extension origin, the companion returns a matching CORS origin together with the minimum methods and headers required by the protocol. `OPTIONS` is allowed without Bearer authentication because it performs no library operation; the subsequent request is still authenticated.

### Protocol v1 endpoints

The v1 surface is intentionally small:

```text
GET  /v1/status
GET  /v1/snapshots
POST /v1/snapshot
GET  /v1/snapshot
```

`GET /v1/status` reports the protocol version and authentication/transport type.

`GET /v1/snapshots` returns only snapshot file names and encrypted-file sizes.

`POST /v1/snapshot` accepts an opaque `application/octet-stream` body plus `X-TabSnap-Name`. The library still sanitizes the final Windows file name and never overwrites an existing snapshot.

`GET /v1/snapshot` returns the encrypted file bytes for one library snapshot selected through `X-TabSnap-Name`.

The protocol rejects chunked/`Transfer-Encoding` requests, duplicate headers, malformed HTTP, unsupported methods and request bodies above 65 MiB. Socket read/write operations use finite timeouts, and responses are marked `Cache-Control: no-store`.

## Removable-drive hint

On Windows the companion asks `GetDriveTypeW` for a best-effort drive classification such as `removable`, `fixed`, `remote` or `optical`.

This value is informational only. In particular, a USB SSD may be reported as `fixed`; TabSnap never uses that result to override the selected storage mode or to decide whether portable storage is allowed.

## Storage configuration

The optional `tabsnap-companion.conf` file stays next to the executable. It records only the storage mode and, for custom mode, the chosen path. It contains no snapshot data, passwords, browser data or authentication secrets.

Deleting the config file returns the companion to portable mode on the next launch.

## Security boundary

M19 introduces the companion's first network listener, but it is restricted to `127.0.0.1` and protected by a fresh high-entropy Bearer token. It is not a cloud service and does not accept Internet or LAN connections.

The listener does not decrypt snapshots, handle browser passwords or expose browser-session content. It only lists names/sizes and moves encrypted `.tabsnap` bytes between an authenticated local client and the existing opaque library.

M20 will add the Chrome-side opt-in bridge that consumes the pairing code while preserving extension-only operation when the companion is absent.
