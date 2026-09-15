# Chrome extension

The Level 1 client is a Manifest V3 Chrome extension.

## Install a prerelease

Download the Chrome ZIP from the matching GitHub prerelease and verify its published `.sha256` file if you want to check the artifact before installing it.

Extract the ZIP, open `chrome://extensions`, enable Developer mode, choose **Load unpacked**, then select the extracted directory containing `manifest.json`.

Prerelease builds are for manual testing and are not published to the Chrome Web Store yet.

## Open TabSnap

Click the TabSnap toolbar action. It opens the TabSnap workspace as a normal extension tab instead of an ephemeral popup.

If a TabSnap workspace tab is already open, another toolbar click focuses that existing tab and its window rather than opening a duplicate.

Because the workflow lives in a normal tab, capture, encryption, decryption and restore are not interrupted merely because a browser popup loses focus.

## Build from source

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

The workspace can:

- encrypt a captured snapshot to a copyable `tabsnap:v1:` string
- download the same encrypted envelope as a `.tabsnap` file
- decrypt either representation locally
- show a preview before restore

Passwords exist only in the extension page's memory and are never stored.

## Current scope

Level 1 targets Chrome. Edge and Firefox are later roadmap levels, as is the optional portable Windows companion for whole-machine workflows.
