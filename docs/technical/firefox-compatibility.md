# Firefox compatibility spike

Target for the first Firefox implementation: **Firefox Desktop 139+**.

Firefox 139 is the minimum because it is the first release with the full `tabGroups` WebExtension API. The `.tabsnap` schema, crypto, compression and companion protocol do not need a Firefox-specific fork.

## Capability matrix

| Capability               | Chromium today                              | Firefox 139+                                            | M26 action                                                  |
| ------------------------ | ------------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------- |
| Tab URL, title, order    | Supported with `tabs`                       | Supported with `tabs`                                   | Shared snapshot fields                                      |
| Pinned tabs              | Supported                                   | Supported                                               | Adapter-specific API calls                                  |
| Active tab               | Supported                                   | Supported                                               | Shared semantics                                            |
| Multiple normal windows  | Supported                                   | Supported                                               | Adapter-specific API calls                                  |
| Window bounds            | Supported                                   | Supported                                               | Preserve best-effort geometry                               |
| Window state             | normal / minimized / maximized / fullscreen | Same core states; API also defines `docked`             | Normalize unsupported `docked` to `normal` with a warning   |
| Focused window           | Supported                                   | Supported                                               | Shared snapshot field                                       |
| Tab-group membership     | Supported                                   | Supported                                               | Canonicalize browser group IDs                              |
| Group title              | Supported                                   | Supported                                               | Shared snapshot field                                       |
| Group color              | 9 shared colors                             | Same 9 colors                                           | Shared snapshot field                                       |
| Group collapsed state    | Supported                                   | Supported                                               | Preserve state, accept browser-specific active-tab behavior |
| Group IDs across restart | Ephemeral                                   | Ephemeral; restored IDs may differ                      | Never persist raw browser IDs                               |
| Private windows          | User opt-in                                 | User-controlled “Run in Private Windows”                | Detect access; never silently claim capture                 |
| Local companion          | Optional `127.0.0.1` permission             | `optional_host_permissions` supported in MV3            | Keep protocol and explicit Connect flow                     |
| Extension background     | MV3 service worker                          | MV3 background script/event page                        | Firefox-specific manifest                                   |
| API namespace            | `chrome.*` today                            | `browser.*` preferred; `chrome.*` supported for porting | Put API access behind Firefox adapter boundary              |
| Store metadata           | Chromium manifest                           | Gecko ID + AMO data-collection declaration              | Firefox-specific manifest/package                           |

## Restore URL policy

Firefox rejects several URLs that Chromium handling alone does not cover. In particular, `tabs.create()` rejects `chrome:`, `javascript:`, `data:`, `file:` and privileged `about:` URLs such as `about:config`, `about:addons` and `about:debugging`.

M26 must therefore use a browser-specific restore policy. The snapshot still keeps the original URL. Restore skips URLs Firefox cannot create and returns a warning instead of deleting data from the snapshot.

`about:blank` is valid. `about:newtab` is special: Firefox documents opening the New Tab page by omitting the `url` property rather than passing `about:newtab` directly.

## Tab groups

Firefox 139+ exposes `tabGroups` plus `tabs.group()` / `tabs.ungroup()`. The group color vocabulary matches Chromium: `blue`, `cyan`, `grey`, `green`, `orange`, `pink`, `purple`, `red`, `yellow`.

One visible semantic difference remains: Firefox can keep the active tab visible inside a collapsed group, while Chrome collapses the group completely and moves the active tab. TabSnap should preserve the requested `collapsed` value and active-tab intent, then accept the browser's native result instead of trying to emulate Chrome UI behavior.

Raw browser group IDs remain runtime-local. TabSnap already maps groups to snapshot-local IDs, which is the correct cross-browser behavior.

## Windows

Firefox exposes window position, size, focus and state. Like Chromium, maximized/minimized/fullscreen creation cannot be combined with explicit bounds. Restore should continue creating geometry first and applying the final non-normal state afterward.

The WebExtensions `WindowState` type also contains `docked`, which is outside the TabSnap v1 schema. If Firefox ever returns it for a captured normal browser window, M26 should normalize it to `normal` rather than expand the file format for a platform-specific edge case.

## Manifest and packaging

The Firefox package needs its own manifest even though most application code remains shared.

The M25 reference manifest is `tests/fixtures/firefox-manifest-v3.json`. CI audits these invariants:

- Manifest V3
- Firefox 139 minimum
- exactly `tabs` + `tabGroups`
- loopback access remains optional
- `background.scripts` uses the existing `background.js`
- no dependency on `background.service_worker`
- a Gecko extension ID exists for MV3 signing
- AMO data collection declaration is `required: ["none"]`
- no content scripts, required host permissions or custom update URL

The fixture ID `tabsnap-spike@invalid.local` is deliberately non-release metadata. M26/M28 must choose the stable production Gecko ID before AMO signing and never change it afterward.

## API boundary for M26

Do not scatter `if (firefox)` checks through the Chromium implementation. M26 should introduce a Firefox adapter/runtime boundary and keep these layers shared:

- `.tabsnap` schema and validation
- canonical serialization
- compression
- encryption
- extension UI where browser behavior is not involved
- companion pairing and encrypted-byte protocol

Firefox-specific code owns browser metadata detection, capture/restore calls, URL restore policy, manifest generation and any API behavior differences.

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
