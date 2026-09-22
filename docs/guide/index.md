# Guide

TabSnap captures browser workspaces, encrypts them locally and gives the encrypted result back to you.

Chrome, Microsoft Edge and Firefox use the same snapshot format. The optional portable Windows companion adds local storage plus coordinated whole-machine capture/restore across explicitly paired browser pages.

## Single-browser flow

1. Open TabSnap in Chrome, Edge or Firefox.
2. Capture the current browser workspace.
3. Review what was captured.
4. Enter a password.
5. Copy an encrypted string or save a `.tabsnap` file.
6. Move it however you want.
7. Import it on another machine or browser.
8. Enter the password.
9. Review and restore.

The browser extension performs encryption and decryption locally.

## Whole-machine flow

1. Run the portable Windows companion with `tabsnap-companion ui`.
2. Press **Start protocol**.
3. Copy the pairing code into the TabSnap pages you want to include.
4. Enter the snapshot password in each paired browser page.
5. Press **Capture machine**.
6. TabSnap stores one `.tabsnap-machine` file containing the encrypted browser payloads.
7. Later, pair the browsers that should receive the restore.
8. Select the machine snapshot and press **Restore selected**.
9. Retry only failed or unavailable targets when needed.

The companion routes encrypted bytes and bounded coordination metadata. Snapshot passwords, keys and decrypted browser workspaces stay inside the extensions.

## Start here

- [Chrome extension](./chrome-extension)
- [Microsoft Edge](./edge-extension)
- [Firefox](./firefox-extension)
- [Windows companion](./windows-companion)
- [Whole-machine companion UI](./windows-companion-ui)
- [Cross-browser compatibility](../technical/cross-browser-compatibility)
- [Privacy](../privacy)
- [Security](../security/)

Nothing requires a TabSnap server or account.
