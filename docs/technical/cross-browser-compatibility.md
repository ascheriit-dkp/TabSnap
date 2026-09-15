# Cross-browser compatibility

TabSnap snapshots are browser-agnostic. Restore is still limited by what the target browser is willing to create.

The rule is simple: **same-browser restore is the fidelity baseline; cross-browser restore is best effort.** TabSnap does not rewrite the encrypted snapshot to hide browser differences.

Before restore, the extension compares the snapshot source with the current browser and reports known non-portable tabs.

## Compatibility matrix

| Source | Target | Common web tabs | Windows/order/pins | Groups | Browser-internal URLs |
| --- | --- | --- | --- | --- | --- |
| Chrome | Edge | Yes | Yes | Yes | Best effort |
| Edge | Chrome | Yes | Yes | Yes | Best effort |
| Chrome / Edge | Firefox | Yes | Yes | Yes | Best effort; some are known unsupported |
| Firefox | Chrome / Edge | Yes | Yes | Yes | Best effort; Firefox-only URLs can be skipped |

## What stays portable

For ordinary `http:` and `https:` tabs, TabSnap preserves the same v1 snapshot fields across Chrome, Edge and Firefox:

- window order
- tab order
- pinned state
- active tab intent
- tab-group membership
- group title, color and collapsed state
- window bounds and supported window state

The `.tabsnap` schema, serialization, compression and encryption are identical regardless of source browser.

## Known feature loss

### Chrome ↔ Edge

Chrome and Edge share the Chromium adapter and workspace model. The main non-portable case is a browser-internal URL: for example, a `chrome:` page restored into Edge or an `edge:` page restored into Chrome.

Those URLs remain in the snapshot. TabSnap warns before restore and lets the target browser reject anything it cannot create.

### Chromium → Firefox

Firefox has a stricter restore policy. Known non-portable examples include:

- Chromium-internal URLs such as `chrome:` or `edge:`
- local `file:` URLs
- extension pages
- privileged pages the Firefox tabs API refuses to create

Supported web tabs are restored normally. Unsupported tabs are counted and reported instead of making the whole restore fail.

### Firefox → Chromium

Firefox-only schemes such as `moz-extension:` and `resource:` are not portable to Chromium. Privileged Firefox `about:` pages such as `about:config` are also treated as non-portable.

`about:blank` is portable. `about:newtab` is handled natively by Firefox and remains best effort when moving to Chromium.

## Groups

The canonical group data is portable, but the browsers do not render every state identically.

In particular, Firefox can keep an active tab visible while its group is collapsed. Chromium may present that state differently. TabSnap preserves the requested group state and active-tab intent, then accepts the target browser's native UI behavior.

This is a visual difference, not a format migration.

## Restore behavior

Cross-browser restore remains non-destructive:

1. the original snapshot is left untouched
2. the preview reports the source browser and target browser
3. known non-portable tabs are counted before restore
4. restore creates what the target browser accepts
5. per-tab failures are returned as warnings

No browser-specific URL is silently converted into a different URL.
