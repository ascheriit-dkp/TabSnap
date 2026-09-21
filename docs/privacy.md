# Privacy policy

Last updated: September 21, 2026.

TabSnap moves browser workspaces between machines without a TabSnap account, backend or sync service.

## Extension-only mode

TabSnap reads the tabs, tab groups and normal browser windows that you explicitly capture. A browser snapshot can contain URLs, tab titles, tab order, pinned/active state, group metadata, window geometry and browser/platform metadata.

That data is processed locally inside the Chrome, Microsoft Edge or Firefox extension. TabSnap does not send it to a TabSnap server, analytics service, advertising service or other remote service.

Passwords are used locally inside the extension to encrypt and decrypt snapshots. TabSnap does not store or transmit the snapshot password.

## Portable Windows companion

The Windows companion is optional. Single-browser capture, encrypted string/file export, import and restore work without it.

The companion protocol is stopped when the native UI opens. When you explicitly start the protocol and connect a browser page, TabSnap requests permission to communicate with `127.0.0.1` on your own machine. On Firefox, companion mode also requires the browser's explicit optional browsing-activity consent before an encrypted browser snapshot can leave the extension.

The companion listener binds only to IPv4 loopback. Each process creates a fresh random session token used by the pairing code. The token is kept in memory and is not written to disk.

TabSnap does not automatically discover browsers, scan browser processes, enumerate browser profiles or contact a cloud service. A browser participates only after its TabSnap page is explicitly paired with the current companion session.

## Connected-browser metadata

While a browser page is paired, the companion keeps a short-lived in-memory registry containing only bounded coordination metadata:

- an ephemeral random browser instance identifier;
- browser kind: Chrome, Edge or Firefox;
- a bounded browser version when available;
- whether that page advertises capture and/or restore capability.

Registry entries expire unless the paired extension page refreshes its lease. This browser-presence registry is not persisted to disk.

The native UI may display this bounded metadata and a shortened instance identifier so the user can see which browser pages are participating.

## Encrypted browser snapshots

A `.tabsnap` file or `tabsnap:v1:` string contains a password-encrypted browser workspace.

Encryption and decryption happen inside the browser extension. The optional companion can store opaque encrypted `.tabsnap` bytes, but it does not receive the password, derived encryption key or decrypted browser state.

Encrypted snapshots may be stored on the local computer, removable storage or another path explicitly chosen by the user. Retention is controlled by the user and the underlying filesystem.

## Whole-machine snapshots

A `.tabsnap-machine` file combines multiple encrypted browser payloads so one capture can cover explicitly paired Chrome, Edge and Firefox pages.

The companion receives each browser payload only after that browser extension has encrypted it. It never decrypts those payloads.

A machine container stores bounded routing metadata alongside the encrypted payloads:

- machine-container format version;
- capture job identifier and persistence time;
- ephemeral source browser instance identifiers;
- browser kind and bounded browser version;
- whether a source capture succeeded or one bounded failure reason;
- encrypted payload lengths/positions.

The machine manifest does **not** contain browser URLs, tab titles, passwords, encryption keys or decrypted workspace contents.

During whole-machine restore, the companion chooses a destination browser and sends only that destination's selected encrypted payload. The destination extension asks for/uses its local password, decrypts locally and performs the restore locally. The companion receives only bounded success/failure state such as password-required, decrypt-failed, restore-failed or timeout.

Completed restore targets are not replayed when another target is retried.

## Data TabSnap does not collect

TabSnap does not collect or transmit to a TabSnap service:

- browsing history outside tabs explicitly included in a capture;
- cookies;
- form contents;
- page contents;
- localStorage or sessionStorage;
- login/session state;
- screenshots;
- snapshot passwords or derived keys;
- analytics or telemetry;
- advertising identifiers;
- usage profiling;
- crash reports.

TabSnap has no TabSnap-operated backend to receive these categories.

## Network access

The extensions have no required web host permission.

Optional companion access is restricted to `http://127.0.0.1/*` and is enabled only through explicit user action/permission. Companion browser endpoints require the current session token and a valid extension origin.

TabSnap does not load remote extension code.

The companion UI's periodic status refresh reads the companion's own in-memory coordination state. It does not scan the LAN, contact the Internet or discover browser processes.

## Third parties

TabSnap does not sell user data and does not share captured browser data with advertisers, analytics providers or data brokers.

If you export an encrypted snapshot or machine container and send it through a third-party storage, messaging or transfer service of your choice, that service is outside TabSnap and subject to its own privacy terms.

Browser extension stores and GitHub may process installation/download metadata under their own terms when you obtain TabSnap from those services. TabSnap itself does not add telemetry for that purpose.

## Security

Browser payloads exported by TabSnap are password-encrypted and authenticated. Lost passwords cannot be recovered by TabSnap because there is no recovery service or server-side key.

The machine-container manifest is routing metadata, not a second encryption layer. The browser contents inside it remain protected by their individual authenticated encrypted TabSnap envelopes.

Security issues can be reported using the instructions in the repository `SECURITY.md`.

## Changes

Material changes to this policy will be committed to the public TabSnap repository with the rest of the project history.

## Contact

Project support and privacy questions can be opened in the public GitHub repository: `ascheriit-dkp/TabSnap`.
