# Level 4 / 1.0 readiness review

Review date: September 21, 2026.

This review covers the implementation state after Level 4 M29–M34.

## Implemented

Level 4 now includes:

- explicit ephemeral Chrome, Edge and Firefox companion registration;
- bounded browser leases and capability metadata;
- coordinated whole-machine capture;
- local browser-side encryption before any snapshot reaches the companion;
- versioned `.tabsnap-machine` containers;
- atomic machine-snapshot persistence;
- strict machine-container inspection and bounded payload reads;
- deterministic coordinated restore routing;
- same-instance, same-browser and cross-browser fallback;
- partial success, skip and retry state;
- no replay of already completed restore targets;
- native Win32 whole-machine UI;
- visible connected-browser/capability state;
- one-window capture, restore and retry workflow;
- hostile container/protocol regression tests;
- multi-job capture/restore isolation tests;
- portable Windows packaging and checksum generation;
- Chrome, Edge and Firefox package generation from the same source tree.

## Automated release gates

The repository CI currently verifies:

- Prettier formatting;
- ESLint;
- TypeScript type checking;
- unit tests;
- Chromium extension build and built-artifact audit;
- Edge package audit;
- Firefox build, audit and `web-ext lint`;
- store-readiness audit;
- documentation build;
- Windows `rustfmt`;
- Windows Clippy with warnings denied;
- Windows companion tests;
- Windows release build;
- real executable smoke test;
- Chromium Playwright extension integration;
- Edge package artifact generation;
- Firefox package artifact generation.

M34 additionally exercises:

- capture job isolation and retention/capacity recovery;
- restore job isolation and destination ownership;
- hostile machine manifests with duplicate ids, invalid metadata/codes/counts and invalid payload lengths;
- coordination/capture HTTP body limits;
- restore completion origin binding.

## Release packaging

The Level 4 prerelease target is `v0.4.0-alpha.1`.

The release workflow produces eight assets:

- Chrome ZIP and SHA-256;
- Edge ZIP and SHA-256;
- Firefox ZIP and SHA-256;
- portable Windows x64 companion ZIP and SHA-256.

The release remains marked as a GitHub prerelease.

## 1.0 decision

**Do not tag `1.0.0` yet.**

Automated coverage is strong, but stable 1.0 still requires manual validation that cannot be honestly inferred from CI alone:

1. install the release candidate manually in current stable Chrome, Edge and Firefox;
2. pair all three browsers with the packaged Windows companion at the same time;
3. perform a real whole-machine capture containing windows, pins, active tabs and groups in each browser;
4. close/reopen or otherwise replace the source browser sessions and perform whole-machine restore;
5. verify same-browser routing and at least one intentional cross-browser fallback;
6. verify partial failure then retry without replaying a completed target;
7. verify wrong-password/decrypt failure remains local to the browser page and is recoverable on retry;
8. run the packaged companion from a normal local folder and from removable storage;
9. verify portable/local/custom storage switching before protocol start;
10. verify the packaged extension permissions/consent surfaces in the actual browser UIs;
11. verify store-ready packages/signing/submission with the real developer accounts.

Store signing/submission and store review are external developer-account actions and are not automated by this repository.

## Promotion criteria

After the manual matrix above is completed without release-blocking defects:

- fix any discovered issues on a release-candidate branch;
- rerun the full CI/release suite;
- update release notes and privacy/security docs if behavior changed;
- publish a final release candidate if needed;
- only then consider changing the product version to `1.0.0`.

Passing CI plus a successful `v0.4.0-alpha.1` publication means **Level 4 prerelease ready**, not automatically **1.0 stable**.
