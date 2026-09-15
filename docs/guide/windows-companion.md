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

## Removable-drive hint

On Windows the companion asks `GetDriveTypeW` for a best-effort drive classification such as `removable`, `fixed`, `remote` or `optical`.

This value is informational only. In particular, a USB SSD may be reported as `fixed`; TabSnap never uses that result to override the selected storage mode or to decide whether portable storage is allowed.

## Storage configuration

The optional `tabsnap-companion.conf` file stays next to the executable. It records only the storage mode and, for custom mode, the chosen path. It contains no snapshot data, passwords, browser data or authentication secrets.

Deleting the config file returns the companion to portable mode on the next launch.

## Security boundary

The M17 companion still has no networking code and no third-party runtime dependency. Storage paths are validated before activation, and the write test is removed immediately after it succeeds.

Later local communication with browser extensions will be loopback-only, versioned and authenticated before it is allowed to move snapshots.
