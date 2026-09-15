# Windows companion UI

M21 adds a native Windows window to the existing portable companion.

Run:

```text
tabsnap-companion ui
```

The CLI commands remain available.

## Snapshot library

The top pane lists regular encrypted `.tabsnap` files from the active library. `Refresh` rescans the directory. `Import` copies an existing `.tabsnap` file into the library using the same M18 validation, size limit, atomic-write and collision-safe naming rules. `Export` copies the selected encrypted snapshot to a chosen destination directory. `Copy path` copies the selected library file path.

The UI never decrypts snapshot contents.

## Storage

Choose `Portable`, `Local` or `Custom`, then press `Apply`.

`Portable` stores snapshots next to the executable. `Local` uses `%LOCALAPPDATA%\TabSnap\snapshots`. `Custom` requires an absolute path in the adjacent text field.

Applying a mode uses the same M17 write test before persisting the selection. The current storage path and Windows drive hint are displayed in the window.

## Local bridge

The protocol is stopped when the UI opens. Press `Start protocol` to launch the M19 authenticated listener on `127.0.0.1` using an OS-selected port and a fresh in-memory session token.

The window then shows the endpoint and pairing code. `Copy pairing code` copies the current code for pasting into the Chrome extension M20 bridge.

The listener has no LAN or Internet bind option. Closing TabSnap stops the process and invalidates the pairing token.

## Privacy

The companion UI handles encrypted `.tabsnap` bytes, file names, paths, storage configuration and the ephemeral local pairing token.

It does not receive the snapshot password. It does not decrypt or inspect tabs, URLs or browser state. No account, cloud backend, telemetry service, WebView or browser content script is introduced by M21.
