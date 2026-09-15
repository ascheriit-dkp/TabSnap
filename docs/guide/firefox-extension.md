# Firefox extension

TabSnap targets **Firefox Desktop 139+**.

Firefox uses the same `.tabsnap` format, encryption, compression, UI and companion protocol as Chrome and Edge. Browser capture and restore behavior is selected through the Firefox adapter at runtime.

## Current status

The Firefox build is an alpha development package.

It includes:

- Firefox runtime detection and `source.browser: "firefox"`
- window/tab capture
- pinned and active-tab state
- tab groups, titles, colors and collapsed state
- window bounds and state
- non-destructive restore
- the optional authenticated localhost companion bridge
- a Firefox-specific Manifest V3 package
- Mozilla `web-ext lint` in CI

The production Gecko extension ID is fixed as `tabsnap@ascheriit-dkp.github.io` so signed updates can retain one identity.

## Differences from Chromium

Firefox rejects some privileged URLs that Chromium may attempt. TabSnap preserves those URLs in the snapshot but skips them during Firefox restore with a warning.

`about:blank` is restored normally. `about:newtab` is restored by asking Firefox to create its native new-tab page without passing an explicit URL.

Firefox may keep the active tab visible in a collapsed tab group. TabSnap preserves the requested collapsed state and active-tab intent, then accepts Firefox's native UI behavior.

If Firefox exposes the WebExtensions `docked` window state, TabSnap normalizes it to `normal` because `.tabsnap` v1 intentionally keeps the shared cross-browser state set.

## Install for development

Build the shared extension and stage the Firefox package:

```bash
pnpm install
pnpm extension:build
pnpm extension:firefox-build
pnpm extension:firefox-audit
pnpm extension:firefox-lint
```

The staged package is written to:

```text
apps/chrome-extension/dist-firefox/
```

For a manual temporary install in Firefox, open `about:debugging`, choose **This Firefox**, choose **Load Temporary Add-on**, then select `apps/chrome-extension/dist-firefox/manifest.json`.

Temporary installs are for development only. AMO signing and the normal Firefox Add-ons distribution flow belong to M28.

## Private windows

Firefox controls whether an extension may run in private windows. TabSnap does not bypass that browser setting. Private-window coverage must be treated as opt-in and must not be advertised as captured unless Firefox granted access.

## Companion

The Windows companion protocol is unchanged. Connecting remains explicit and requests only optional access to `http://127.0.0.1/*`. The pairing token stays in memory for the current extension page session.
