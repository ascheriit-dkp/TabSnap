# ADR 0014 — Separate Edge release artifact from one Chromium bundle

## Status

Accepted.

## Context

Chrome and Microsoft Edge use the same TabSnap Chromium implementation. Maintaining a second source tree for Edge would duplicate capture, restore, encryption and companion code without adding a different runtime capability.

Microsoft Edge Add-ons still needs a browser-specific release artifact and certification path.

## Decision

Build Chrome and Edge release ZIPs from the same audited `apps/chrome-extension/dist` directory.

Before an Edge package is accepted by CI or release automation, an Edge-specific audit verifies that:

- the manifest is MV3
- the name and description are not Chrome-branded
- `update_url` is absent
- `minimum_chrome_version` is used for the shared Chromium minimum version
- required permissions remain exactly `tabs` and `tabGroups`
- loopback access remains an optional host permission
- no content script, externally-connectable endpoint or required host permission is introduced

CI also runs the shared extension under an Edge user agent and verifies Edge source metadata plus companion pairing.

## Consequences

Chrome and Edge artifacts can be released independently while remaining byte-for-byte equivalent when no store-specific manifest change is required.

A store-specific manifest fork may be introduced later only if a concrete Edge Add-ons requirement requires it.

Automated Chromium tests do not replace a manual sideload check in Microsoft Edge before store submission.
