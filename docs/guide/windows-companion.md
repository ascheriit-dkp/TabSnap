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

The companion does not decrypt, decompress or parse `.tabsnap` payloads in M18. Import and export copy the encrypted bytes unchanged, and the internal write API accepts an already-encrypted byte sequence for the future browser bridge.

Passwords therefore remain solely in the browser workflow at this milestone. The companion cannot determine the tabs, URLs or other contents of a stored snapshot.

## Removable-drive hint

On Windows the companion asks `GetDriveTypeW` for a best-effort drive classification such as `removable`, `fixed`, `remote` or `optical`.

This value is informational only. In particular, a USB SSD may be reported as `fixed`; TabSnap never uses that result to override the selected storage mode or to decide whether portable storage is allowed.

## Storage configuration

The optional `tabsnap-companion.conf` file stays next to the executable. It records only the storage mode and, for custom mode, the chosen path. It contains no snapshot data, passwords, browser data or authentication secrets.

Deleting the config file returns the companion to portable mode on the next launch.

## Security boundary

The M18 companion still has no networking code and no third-party runtime dependency. Storage paths are validated before activation, library names cannot escape their selected directories, and encrypted snapshot payloads remain opaque at rest.

Later local communication with browser extensions will be loopback-only, versioned and authenticated before it is allowed to move snapshots.
