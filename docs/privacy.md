# Privacy policy

Last updated: September 16, 2026.

TabSnap moves browser workspaces between machines without a TabSnap account or backend.

## Extension-only mode

TabSnap reads the tabs, tab groups and normal browser windows that you explicitly capture. A snapshot can contain URLs, tab titles, tab order, pinned/active state, group metadata, window geometry and browser/platform metadata.

That data is processed locally inside the extension. TabSnap does not send it to a TabSnap server, analytics service, advertising service or any other remote service.

Passwords are used locally to encrypt and decrypt snapshots. TabSnap does not store or transmit your password.

## Portable companion

The Windows companion is optional. Extension-only capture, encrypted string/file export, import and restore work without it.

When you explicitly connect the companion, TabSnap requests permission to communicate with `127.0.0.1` on your own machine. On Firefox, companion mode also requires an explicit optional browsing-activity data consent before any encrypted browser snapshot can be sent outside the extension.

Snapshots are encrypted inside the extension before they are sent to the companion. The companion receives and stores opaque encrypted `.tabsnap` bytes. It does not receive the snapshot password and does not decrypt browser state.

The companion may store encrypted snapshots on the local computer, removable storage or another path chosen by the user. Retention is controlled by the user through the companion and the underlying filesystem.

## Data TabSnap does not collect

TabSnap does not collect or transmit:

- browsing history outside the tabs included in a capture
- cookies
- form contents
- page contents
- localStorage or sessionStorage
- login/session state
- screenshots
- analytics or telemetry
- advertising identifiers
- crash reports to a TabSnap service

## Network access

The extension has no required host permissions. Optional companion communication is restricted to IPv4 loopback (`127.0.0.1`) and is enabled only after an explicit user action.

TabSnap does not load remote code.

## Third parties

TabSnap does not sell user data and does not share captured browser data with advertisers, analytics providers or data brokers.

If a user exports an encrypted snapshot and sends it through a third-party service of their choice, that service is outside TabSnap and subject to that service's own privacy terms.

## Security

Snapshots exported by TabSnap are password-encrypted and authenticated. Lost passwords cannot be recovered by TabSnap because there is no recovery service or server-side key.

Security issues can be reported using the instructions in the repository `SECURITY.md`.

## Changes

Material changes to this policy will be committed to the public TabSnap repository with the rest of the project history.

## Contact

Project support and privacy questions can be opened in the public GitHub repository: `ascheriit-dkp/TabSnap`.
