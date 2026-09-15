# Firefox compatibility

TabSnap targets **Firefox Desktop 139+**. Firefox 139 is the minimum because it is the first release with the full `tabGroups` WebExtension API.

The `.tabsnap` schema, validation, serialization, compression, encryption and companion protocol are shared with Chrome and Edge. Firefox-specific behavior is isolated behind the Firefox adapter and its own Manifest V3 package.

## Capability matrix

| Capability               | Chromium                                    | Firefox 139+                                | TabSnap behavior                             |
| ------------------------ | ------------------------------------------- | ------------------------------------------- | -------------------------------------------- |
| Tab URL, title, order    | Supported                                   | Supported                                   | Shared snapshot fields                       |
| Pinned tabs              | Supported                                   | Supported                                   | Shared restore engine                        |
| Active tab               | Supported                                   | Supported                                   | Shared restore engine                        |
| Multiple normal windows  | Supported                                   | Supported                                   | Shared restore engine                        |
| Window bounds            | Supported                                   | Supported                                   | Preserve best-effort geometry                |
| Window state             | normal / minimized / maximized / fullscreen | Same core states; API also defines `docked` | Firefox normalizes `docked` to `normal`      |
| Focused window           | Supported                                   | Supported                                   | Shared snapshot field                        |
| Tab-group membership     | Supported                                   | Supported                                   | Browser IDs become snapshot-local IDs        |
| Group title              | Supported                                   | Supported                                   | Shared snapshot field                        |
| Group color              | 9 shared colors                             | Same 9 colors                               | Shared snapshot field                        |
| Group collapsed state    | Supported                                   | Supported                                   | Preserve state; accept native UI differences |
| Group IDs across restart | Ephemeral                                   | Ephemeral                                   | Never persist raw browser IDs                |
| Private windows          | User opt-in                                 | User-controlled “Run in Private Windows”    | Never bypass browser policy                  |
| Local companion          | Optional loopback permission                | Optional loopback permission                | Same authenticated protocol                  |
| Extension background     | MV3 service worker                          | MV3 background script/event page            | Separate manifests                           |
| Store metadata           | Chromium manifest                           | Gecko ID + AMO declaration                  | Separate Firefox package                     |

## Adapter architecture

`browser.ts` is only a runtime dispatcher. Capture and restore are implemented once by the shared WebExtension adapter.

Chromium and Firefox supply small configuration layers for:

- browser/source metadata
- restore URL policy
- captured window-state normalization

This keeps ordering, groups, pinning, focus, geometry, partial-failure handling and non-destructive restore in one implementation.

## Restore URL policy

Firefox rejects several URLs that Chromium may attempt, including `chrome:`, `javascript:`, `data:`, `file:` and privileged `about:` pages such as `about:config`, `about:addons` and `about:debugging`.

TabSnap keeps the original URL in the snapshot. Firefox restore skips unsupported URLs and reports a warning instead of deleting data from the snapshot.

`about:blank` is restored normally. `about:newtab` is restored by omitting the `url` property so Firefox creates its native new-tab page.

## Tab groups

Firefox 139+ exposes `tabGroups` plus `tabs.group()` / `tabs.ungroup()`. The group color vocabulary matches Chromium: `blue`, `cyan`, `grey`, `green`, `orange`, `pink`, `purple`, `red`, `yellow`.

Firefox can keep the active tab visible inside a collapsed group while Chromium may move the active tab. TabSnap preserves the requested `collapsed` value and active-tab intent, then accepts the browser's native result.

Raw group IDs remain runtime-local and are never serialized directly.

## Windows

Firefox exposes window position, size, focus and state. TabSnap creates geometry first and applies a final non-normal state afterward, matching the existing Chromium restore path.

The WebExtensions `WindowState` type also contains `docked`, which is outside the `.tabsnap` v1 cross-browser state set. Firefox capture normalizes that value to `normal`.

## Manifest and packaging

The production Firefox manifest is `apps/chrome-extension/firefox-manifest.json`.

The package is staged in `apps/chrome-extension/dist-firefox/` from the same application bundle used by Chromium, with the Firefox manifest replacing the Chromium manifest.

CI verifies:

- Manifest V3
- Firefox 139 minimum
- exactly `tabs` + `tabGroups`
- optional localhost companion access only
- `background.scripts` and no service worker dependency
- stable Gecko ID `tabsnap@ascheriit-dkp.github.io`
- AMO data collection declaration `required: ["none"]`
- no content scripts, required host permissions or custom update URL
- Mozilla `web-ext lint`
- deterministic Firefox ZIP + SHA-256 artifact

## Automated test boundary

The Firefox runtime detector and browser-specific policies are unit-tested. The shared capture/restore implementation continues to be exercised end-to-end through Chromium/Edge integration tests.

Playwright's documented extension loading flow is Chromium-only, so CI does not pretend that a Chromium process is a real Firefox WebExtension runtime. A manual Firefox temporary-install check is required before AMO submission in M28.

## Sources

- Firefox 139 release notes: https://developer.mozilla.org/en-US/docs/Mozilla/Firefox/Releases/139
- `tabGroups`: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/tabGroups
- `tabGroups.TabGroup`: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/tabGroups/TabGroup
- `tabGroups.Color`: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/tabGroups/Color
- Tabs API permissions: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Working_with_the_Tabs_API
- `tabs.create()`: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/tabs/create
- `windows.create()`: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/windows/create
- `windows.WindowState`: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/windows/WindowState
- MV3 background differences: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/background
- Firefox-specific manifest settings: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/browser_specific_settings
- Optional host permissions: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/optional_host_permissions
- Private browsing: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/incognito
- Cross-browser extensions: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Build_a_cross_browser_extension
