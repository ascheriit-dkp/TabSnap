# Windows companion UI

The portable companion includes a native Win32 UI in the same executable.

Run:

```text
tabsnap-companion ui
```

The CLI commands remain available.

## Browser snapshot library

The left library pane lists regular encrypted `.tabsnap` files from the active storage directory. `Refresh` rescans the directory. `Import` copies an existing `.tabsnap` file into the library using the existing validation, size limit, atomic-write and collision-safe naming rules. `Export` copies the selected encrypted snapshot to a chosen destination directory. `Copy path` copies the selected library file path.

The UI never decrypts snapshot contents.

## Whole-machine snapshot library

The right library pane lists valid `.tabsnap-machine` containers from the same active storage directory.

Each row shows the file name, number of successful encrypted browser payloads and total container size. Invalid, oversized, non-regular or symbolic-link entries are not offered as restore choices.

A successful whole-machine capture automatically writes a new versioned machine container and refreshes this list.

## Storage

Choose `Portable`, `Local` or `Custom`, then press `Apply`.

`Portable` stores snapshots next to the executable. `Local` uses `%LOCALAPPDATA%\TabSnap\snapshots`. `Custom` requires an absolute path in the adjacent text field.

Applying a mode uses the existing write test before persisting the selection. The current storage path and Windows drive hint are displayed in the window.

Storage cannot be changed after the local protocol has started in the current process. Restart the companion first. This keeps the running browser coordination controls and both snapshot libraries bound to one storage root.

## Local bridge and connected browsers

The protocol is stopped when the UI opens. Press `Start protocol` to launch the authenticated listener on `127.0.0.1` using an OS-selected port and a fresh in-memory session token.

The window then shows the endpoint and pairing code. `Copy pairing code` copies the current code for pasting into TabSnap browser pages.

Browser discovery is not automatic. A Chrome, Edge or Firefox page appears only after the user explicitly pairs it with this session. The connected-browser pane then shows its browser kind, bounded version, advertised capture/restore capabilities and a shortened ephemeral instance id.

The UI refreshes this view from the companion's in-memory registry. The timer does not scan processes, enumerate browser profiles or contact the network.

The listener has no LAN or Internet bind option. Closing TabSnap stops the process and invalidates the pairing token.

## Whole-machine capture

1. Start the protocol.
2. Pair the browser pages that should participate.
3. Enter the snapshot password in each paired browser page.
4. Press `Capture machine`.

The progress pane shows each targeted browser as pending, capturing, complete or failed.

Every browser captures and encrypts its own workspace locally. The companion receives only encrypted TabSnap payloads plus bounded coordination metadata. When the job becomes terminal, at least one successful payload is required before the UI writes a `.tabsnap-machine` file.

Partial capture is explicit: successful browser payloads are stored and failed targets remain recorded in the machine manifest.

## Whole-machine restore

Select a `.tabsnap-machine` entry and press `Restore selected`.

The companion maps each successful source payload to a currently connected restore-capable browser using the M32 routing rules: same live instance first, then same browser, then cross-browser fallback.

The progress pane shows the source browser, chosen destination and per-target state.

The destination browser receives only its selected encrypted payload, decrypts it locally with the password entered in that browser page and uses the existing TabSnap restore adapter.

If a destination is unavailable, the target is shown as skipped. Password, decrypt, restore and timeout failures are shown per target. When retryable targets remain, connect or correct the required browser page and press `Retry restore`. Already completed targets are not replayed.

## Privacy

The companion UI handles encrypted `.tabsnap` and `.tabsnap-machine` bytes, bounded browser coordination metadata, file names, paths, storage configuration and the ephemeral local pairing token.

It does not receive snapshot passwords or encryption keys. It does not decrypt or inspect tabs, URLs, titles or browser state. No account, cloud backend, telemetry service, WebView, browser-process discovery or hidden background network access is introduced by the whole-machine UI.
