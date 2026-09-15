# Windows companion

Level 2 adds an optional portable Windows companion.

The companion is not required for the Chrome-only Level 1 workflow. It exists to make local snapshot storage and later browser coordination easier without adding a cloud service.

## Portable layout

M16 derives storage from the executable location:

```text
TabSnap/
├── tabsnap-companion.exe
└── snapshots/
```

If that directory lives on a USB drive, the default snapshot library lives on the same drive automatically.

The M16 executable does not install a Windows service, write machine-wide registry keys, copy itself into `Program Files`, or require administrator privileges.

## Current commands

```text
tabsnap-companion info
tabsnap-companion init
```

`info` prints the executable, portable root and snapshot-library paths.

`init` creates only the `snapshots` directory next to the executable.

This CLI is the foundation used to test portable filesystem behavior. A native companion UI is a later Level 2 milestone.

## Storage policy

M16 deliberately uses one deterministic rule: storage is next to the executable.

M17 will add explicit portable, local and custom modes plus Windows removable-drive hints. TabSnap will not assume that every external USB SSD is reported by Windows as a removable drive.

## Security boundary

The M16 companion has no networking code and no third-party runtime dependency. It only discovers its own executable path and creates a local directory when asked.

Later local communication with browser extensions will be loopback-only, versioned and authenticated before it is allowed to move snapshots.
