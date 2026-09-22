# Store submission

TabSnap ships one shared application with browser-specific packages for Chrome, Microsoft Edge and Firefox.

This page is the canonical submission checklist. Store dashboards are still external systems; do not treat a successful GitHub release as a store publication.

## Canonical listing

**Name:** TabSnap

**Short description:** Move a browser workspace between machines without a backend.

**Single purpose:** Capture the current browser workspace, encrypt it locally, move it to another machine, and restore it.

**Long description:**

TabSnap captures browser windows, tabs, tab order, pinned tabs, tab groups and best-effort window layout, then encrypts the snapshot locally with a password.

Export it as an encrypted string or `.tabsnap` file, move it using any channel you choose, and import it on another machine. Chrome, Microsoft Edge and Firefox use the same open snapshot format. Cross-browser restore is best-effort and reports known browser-specific tabs before restore.

No TabSnap account. No TabSnap backend. No analytics. No telemetry.

An optional portable Windows companion can keep encrypted snapshots on the local computer or removable storage and coordinate whole-machine capture/restore across explicitly paired Chrome, Edge and Firefox pages. Companion mode is opt-in and uses authenticated IPv4 loopback only.

## Permission justifications

### `tabs`

Required to enumerate the tabs being captured and read the URL, title, order, pinned state and active state needed to reproduce a workspace. It is also used to create, activate and pin tabs during restore.

TabSnap does not use this permission to collect browsing history outside the tabs explicitly included in a capture.

### `tabGroups`

Required to read and recreate tab-group membership, title, color and collapsed state. Raw browser group IDs are never persisted; TabSnap converts them to snapshot-local IDs.

### Optional `http://127.0.0.1/*`

Used only when the user explicitly connects the optional portable Windows companion. The extension has no required host permission. Companion traffic is restricted to IPv4 loopback on the user's own machine and authenticated with an in-memory session bearer token.

### Firefox optional `browsingActivity` data collection

Extension-only mode requires no data transmission outside the extension.

Firefox companion mode is optional. A snapshot contains browsing activity such as URLs, so Firefox declares `browsingActivity` as optional data collection. The extension requests that consent only when the user presses Connect. If consent is declined, companion mode remains disabled and extension-only capture/export/import/restore continues to work.

Snapshots are encrypted before transmission to the companion. The companion never receives the snapshot password and stores opaque encrypted bytes.

## Remote code

TabSnap does not load or execute remote code. Application JavaScript and WASM are packaged with the extension. The CSP permits local scripts and packaged WASM only.

## Privacy answers

- TabSnap account required: no
- TabSnap backend: no
- analytics/telemetry: no
- ads: no
- sale of user data: no
- required host access: none
- optional network access: IPv4 loopback companion only
- passwords transmitted: no
- decrypted snapshots transmitted to companion: no
- privacy policy: `docs/privacy.md`

Public privacy policy URL for submission:

`https://github.com/ascheriit-dkp/TabSnap/blob/main/docs/privacy.md`

## Reviewer test path

The companion is not required to review the primary extension function.

1. Install the browser-specific package.
2. Click the TabSnap toolbar action. It opens the persistent TabSnap page.
3. Press Capture.
4. Enter a test password.
5. Export an encrypted string or `.tabsnap` file.
6. Import the result with the same password.
7. Review the preview.
8. Press Restore. Restore is non-destructive and creates new windows alongside the existing workspace.

Optional companion review:

1. Start the portable Windows companion.
2. Copy its one-time pairing code into TabSnap.
3. Press Connect and approve the optional permission prompt(s).
4. Save an encrypted snapshot to the companion.
5. Load it back and decrypt it in the extension.

## Chrome Web Store

Before submission:

- use the Chrome ZIP from a tagged release
- provide store icon and screenshots in the developer dashboard
- complete the Privacy tab using the disclosures above
- justify `tabs`, `tabGroups` and optional loopback access
- declare no remote code
- verify the developer account satisfies current Chrome Web Store account/security requirements
- perform a manual sideload smoke test in current Chrome stable

## Microsoft Edge Add-ons

Before submission:

- use the Edge ZIP from the same tagged release
- complete Properties, Privacy, Store listings and Certification notes
- use the same single-purpose and permission justifications above
- provide required logo/screenshots in Partner Center
- perform a manual sideload smoke test in current Microsoft Edge stable

## Firefox Add-ons (AMO)

Before submission:

- use the Firefox ZIP from the tagged release
- use Firefox Desktop 140+; this is the minimum for the built-in data-transmission consent used by optional companion mode
- keep the stable Gecko ID `tabsnap@ascheriit-dkp.github.io`
- declare `required: ["none"]` and optional `browsingActivity`
- run `pnpm extension:firefox-lint`
- perform a temporary-install smoke test in current Firefox stable
- upload source code for review because the submitted extension is bundled/transpiled
- include the reproducible build instructions below in reviewer notes

### AMO source build

Use the exact source commit matching the submitted package.

```bash
corepack enable
pnpm install --frozen-lockfile
pnpm extension:build
pnpm extension:firefox-build
pnpm extension:firefox-audit
pnpm extension:firefox-lint
```

Expected Firefox output: `apps/chrome-extension/dist-firefox/`.

Runtime/tool versions are pinned by `package.json`, `pnpm-lock.yaml` and the repository CI. Third-party JavaScript dependencies are declared in the workspace manifests and lockfile.

A convenient source archive for AMO is the complete Git archive of the exact release tag:

```bash
git archive --format=zip --output tabsnap-source.zip <release-tag>
```

## Manual blockers

GitHub automation cannot complete these external account actions:

- Chrome Web Store developer registration / submission
- Microsoft Partner Center registration / submission
- Mozilla AMO developer submission / signing
- dashboard screenshots and final store-specific visual assets
- store review responses

Everything else needed for review should live in the repository and tagged release.
