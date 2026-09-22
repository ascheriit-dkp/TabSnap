# Manual release QA

Use this checklist for the final packaged-browser and removable-storage validation before promoting TabSnap to stable `1.0.0`.

Prefer fresh test browser profiles. Do not use production sessions containing sensitive tabs unless you intentionally want them in the test snapshot.

## Prepare the published release

From a Windows checkout of the exact release source:

```powershell
pwsh ./scripts/prepare-manual-release-qa.ps1
```

To test a specific tag or destination:

```powershell
pwsh ./scripts/prepare-manual-release-qa.ps1 -Tag v0.4.0-alpha.1 -Destination C:\Temp\tabsnap-qa
```

The preparer downloads the four release archives plus their four SHA-256 files, verifies every checksum, checks package versions, extracts the browser packages and companion, then copies this checklist into the QA workspace.

Record the environment before starting:

- Windows version:
- Chrome version:
- Edge version:
- Firefox version:
- Release tag:
- QA machine:
- Removable-storage device:
- Notes:

## QA-01 — Install the exact published packages

Chrome:

1. Open `chrome://extensions/`.
2. Enable Developer mode.
3. Choose **Load unpacked**.
4. Select `expanded/chrome`.

Edge:

1. Open `edge://extensions/`.
2. Enable Developer mode.
3. Choose **Load unpacked**.
4. Select `expanded/edge`.

Firefox:

1. Open `about:debugging`.
2. Choose **This Firefox**.
3. Choose **Load Temporary Add-on**.
4. Select `expanded/firefox/manifest.json`.

Expected:

- [ ] Chrome loads without manifest/runtime errors.
- [ ] Edge loads without manifest/runtime errors.
- [ ] Firefox loads without manifest/runtime errors.
- [ ] Toolbar action opens/focuses the persistent TabSnap page in all three browsers.
- [ ] No browser shows unexpected required host access.

Evidence / notes:

## QA-02 — Single-browser round trip

Run once in each browser.

Create a workspace containing:

- at least two normal windows;
- ordinary HTTPS tabs;
- one pinned tab;
- one active tab that is not first;
- at least one tab group with title/color/collapsed state where supported.

For each browser:

1. Capture.
2. Review the preview.
3. Encrypt with a test password.
4. Export a `.tabsnap` file.
5. Import the same file.
6. Enter the password.
7. Restore.

Expected:

- [ ] Chrome round trip passes.
- [ ] Edge round trip passes.
- [ ] Firefox round trip passes.
- [ ] Restore is non-destructive.
- [ ] Window/tab order, pinning, active-tab intent and groups are preserved within documented browser limits.
- [ ] Wrong or unsupported URLs are reported instead of aborting the whole restore.

Evidence / notes:

## QA-03 — Pair all three browsers

1. Run `expanded/companion/tabsnap-companion.exe ui`.
2. Confirm the protocol is initially stopped.
3. Press **Start protocol**.
4. Copy the pairing code.
5. Pair Chrome, Edge and Firefox TabSnap pages with the same running companion.

Expected:

- [ ] All three appear in the companion UI.
- [ ] Browser kind/version/capabilities are correct.
- [ ] No browser appears before explicit pairing.
- [ ] Chrome and Edge request only optional loopback access when connecting.
- [ ] Firefox requests its optional browsing-activity consent and loopback access only when connecting.
- [ ] Declining optional permission leaves extension-only mode usable.

Evidence / notes:

## QA-04 — Whole-machine capture

With all three browsers paired, create distinct test workspaces in each and enter the same test password in each TabSnap page.

Press **Capture machine**.

Expected:

- [ ] Per-browser progress is visible.
- [ ] Each successful browser reaches complete.
- [ ] A new `.tabsnap-machine` file appears automatically.
- [ ] The machine snapshot reports the expected successful/failed target counts.
- [ ] No password is requested by the companion itself.

Record:

- Machine snapshot file:
- Size:
- SHA-256:

Evidence / notes:

## QA-05 — Same-browser whole-machine restore

Replace or close the source test workspaces so the restore result is obvious. Pair Chrome, Edge and Firefox again, select the saved machine snapshot and press **Restore selected**.

Expected:

- [ ] Chrome source payload routes to Chrome.
- [ ] Edge source payload routes to Edge.
- [ ] Firefox source payload routes to Firefox.
- [ ] Per-target progress is visible.
- [ ] All expected targets complete.
- [ ] Restore remains non-destructive.
- [ ] Each browser decrypts locally using the password entered in its own TabSnap page.

Evidence / notes:

## QA-06 — Cross-browser fallback

Use a machine snapshot containing a source payload whose same-browser destination is deliberately unavailable. Keep another supported browser paired.

Expected:

- [ ] The source is routed to an unused cross-browser destination.
- [ ] Ordinary web tabs restore.
- [ ] Known browser-specific/privileged URLs are warned/skipped according to the compatibility documentation.
- [ ] No source payload is silently rewritten.

Record source -> destination:

Evidence / notes:

## QA-07 — Partial restore failure and retry

Start a whole-machine restore with at least two destinations available.

Cause one destination to fail locally, for example by using a wrong password there, while allowing another destination to complete.

Expected:

- [ ] Successful target remains complete.
- [ ] Failed target shows a bounded failure state.
- [ ] Retry becomes available.
- [ ] Correct the failed destination and retry.
- [ ] Failed target completes on retry.
- [ ] Previously completed target is not replayed.

Evidence / notes:

## QA-08 — Password/decrypt boundary

Use a wrong password for one encrypted payload.

Expected:

- [ ] Decrypt fails in the browser page.
- [ ] Companion does not display or receive the password.
- [ ] Companion reports only bounded failure state.
- [ ] Correct password succeeds on retry where retry is supported.

Evidence / notes:

## QA-09 — Storage modes before protocol start

Restart the companion so the protocol is stopped.

Test:

1. Portable storage.
2. Local storage.
3. Custom absolute path.

Expected:

- [ ] Every mode is write-tested before activation.
- [ ] Portable path resolves beside the executable.
- [ ] Local path resolves under `%LOCALAPPDATA%\TabSnap\snapshots`.
- [ ] Custom relative paths are rejected.
- [ ] Changing storage after protocol start is rejected until restart.

Evidence / notes:

## QA-10 — Removable-storage run

Copy the complete companion package to a removable drive and run it from there in portable mode.

Expected:

- [ ] No installer is required.
- [ ] No administrator privileges are required.
- [ ] Portable snapshot directory is created beside the executable on that drive.
- [ ] Capture can persist there.
- [ ] Restore can read the resulting machine snapshot there.
- [ ] A USB SSD being reported as `fixed` is treated only as a drive hint and does not block portable mode.

Drive letter/path:

Reported drive hint:

Evidence / notes:

## QA-11 — Session lifecycle

With browsers paired:

1. Note the current pairing code.
2. Close the companion completely.
3. Start it again.

Expected:

- [ ] Old browser registrations disappear.
- [ ] Old pairing token/code no longer authenticates.
- [ ] New process exposes a fresh pairing code.
- [ ] Browsers must explicitly pair again.

Evidence / notes:

## QA-12 — Final permission/store surfaces

Inspect the actual loaded packages and browser permission prompts.

Expected:

- [ ] Chrome required permissions are only `tabs` + `tabGroups`.
- [ ] Edge required permissions are only `tabs` + `tabGroups`.
- [ ] Firefox required permissions are only `tabs` + `tabGroups`.
- [ ] `http://127.0.0.1/*` remains optional.
- [ ] Firefox shows the intended optional `browsingActivity` consent for companion use.
- [ ] No content scripts or unexpected host permissions appear.

Evidence / notes:

## External store actions

These cannot be completed by repository CI:

- [ ] Chrome Web Store developer account/submission path checked.
- [ ] Microsoft Edge Add-ons Partner Center submission path checked.
- [ ] Mozilla AMO submission/signing path checked.
- [ ] Required icons/screenshots/listing fields available.
- [ ] Firefox source archive/reproducible build material accepted by AMO workflow.

## Final result

Release tag:

- [ ] PASS — no release-blocking defect found.
- [ ] FAIL — release-blocking defect(s) listed below.

Release blockers:

Non-blocking notes:

Only a PASS here, plus green repository/release CI, is enough to consider promoting the project to `1.0.0`.
