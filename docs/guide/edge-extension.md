# Microsoft Edge extension

TabSnap uses one Chromium codebase for Chrome and Microsoft Edge.

The Edge package is built from the same audited extension bundle as Chrome. Runtime detection records Edge snapshots as `source.browser: edge` and uses the `Edg/<version>` browser version when available.

## Install the prerelease manually

Download the Edge ZIP from the GitHub release, verify its `.sha256` file, then extract it.

In Microsoft Edge:

1. open `edge://extensions/`
2. enable **Developer mode**
3. choose **Load unpacked**
4. select the extracted TabSnap directory

The unpacked package requests only the normal `tabs` and `tabGroups` permissions. Access to `http://127.0.0.1/*` remains optional and is requested only when you explicitly connect the portable companion.

## Compatibility

Chrome and Edge share the same capture and restore implementation for normal windows, tabs, pinned state, active tabs, tab groups and window state.

Browser-owned pages such as `edge://` remain best-effort. TabSnap records them, lets Edge attempt to restore them and reports failures instead of silently discarding them.

`edge-extension:` URLs are never restored.

## Portable companion

The Windows companion protocol is unchanged for Edge. The extension still connects only to the paired IPv4 loopback endpoint, and the pairing token remains process-scoped.

The companion never receives the snapshot password and never decrypts browser state.

## Automated testing boundary

The shared Chromium implementation is exercised automatically under an Edge user agent, including capture metadata and companion pairing. A manual sideload check in the current Microsoft Edge build is still required before an Edge Add-ons submission.
