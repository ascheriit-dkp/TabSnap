# Chrome extension

The Level 1 client is a Manifest V3 Chrome extension.

## Build

```bash
pnpm install
pnpm extension:build
```

Load `apps/chrome-extension/dist` as an unpacked extension from `chrome://extensions`.

## Permissions

The extension requests only:

- `tabs` — read tab URLs and titles for capture
- `tabGroups` — read and restore group metadata

There are no host permissions and no network permissions.

The extension CSP allows packaged WebAssembly because Argon2id is implemented locally with WASM. Remote scripts are not allowed.

## Capture

Capture includes normal Chrome windows, their geometry and state when available, tabs, tab order, active tab, pinning and tab groups.

Chrome group IDs only exist for one browser session. TabSnap replaces them with portable IDs based on group order inside each window.

Window ordering is best-effort because Chrome does not expose a stable global z-order API.

## Restore

Restore validates the snapshot before creating anything.

It creates new windows and does not close the existing workspace.

Tabs are restored independently. Invalid or Chrome-rejected URLs are skipped and reported instead of aborting the entire restore.

Groups, pinning, active tabs, window geometry and window state are applied after tabs are created.

## Export and import

The popup can:

- encrypt a captured snapshot to a copyable `tabsnap:v1:` string
- download the same encrypted envelope as a `.tabsnap` file
- decrypt either representation locally
- show a preview before restore

Passwords exist only in the popup's memory and are never stored.

## Current limitation

Long operations run in the extension popup. Keep the popup open until capture, encryption, decryption or restore completes. A persistent full-page workflow can replace this in a later hardening milestone.
