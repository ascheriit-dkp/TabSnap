# Reviewer notes

## What TabSnap does

TabSnap has one purpose: move a browser workspace between machines. It captures the user's currently open browser workspace, encrypts the snapshot locally, and restores it later.

The extension does not require an account or remote backend.

## Basic review path

The optional Windows companion is not required.

1. Install the submitted extension package.
2. Click the TabSnap toolbar action.
3. Press **Capture**.
4. Enter any test password.
5. Export an encrypted string or `.tabsnap` file.
6. Import the exported result using the same password.
7. Review the displayed snapshot summary.
8. Press **Restore**.

Restore is non-destructive: TabSnap creates restored windows alongside the reviewer's current workspace.

## Permissions

`tabs` is used to read and recreate the tabs included in the workspace, including URL, title, order, active state and pinned state.

`tabGroups` is used to read and recreate tab-group membership and metadata.

`http://127.0.0.1/*` is optional and is requested only when the reviewer explicitly chooses to connect the optional portable companion.

There are no required host permissions and no content scripts.

## Network / remote code

The normal extension workflow performs no remote network requests. TabSnap does not load remote JavaScript, WASM, fonts or other executable code.

Optional companion communication is restricted to IPv4 loopback and uses an authenticated session token. The companion is a separate open-source Windows executable in the same repository.

## Data handling

Extension-only mode processes captured browser state locally. Passwords remain inside the extension.

The optional companion receives only encrypted `.tabsnap` bytes. It does not receive the password and cannot decrypt the stored snapshot.

Firefox declares no required data collection and declares `browsingActivity` as optional for companion mode. Firefox requests this optional data permission only when the user presses Connect. Declining it leaves companion mode disabled while extension-only features remain available.

Privacy policy:
https://github.com/ascheriit-dkp/TabSnap/blob/main/docs/privacy.md

## Firefox source build

The submitted Firefox package is bundled from TypeScript source. Review source should be taken from the exact Git tag matching the submitted version.

Requirements:

- Node.js 24
- pnpm 11.25.0

Build commands:

```bash
corepack enable
pnpm install --frozen-lockfile
pnpm extension:build
pnpm extension:firefox-build
pnpm extension:firefox-audit
pnpm extension:firefox-lint
```

The final Firefox directory is `apps/chrome-extension/dist-firefox/`.

Third-party JavaScript dependencies and exact resolved versions are declared in `package.json`, workspace package manifests and `pnpm-lock.yaml`.
