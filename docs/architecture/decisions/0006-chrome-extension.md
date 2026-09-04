# ADR 0006: Chrome MV3 extension adapter

Status: accepted

## Decision

Implement Level 1 as a Chrome Manifest V3 extension with a browser-specific adapter around the browser-agnostic snapshot schema.

The extension requests `tabs` and `tabGroups` only. It does not request host permissions.

## Capture

Use `chrome.windows.getAll({ populate: true })` for normal windows and `chrome.tabGroups.query()` for group metadata.

Chrome group IDs are session-local, so capture replaces them with deterministic per-window snapshot IDs.

Group order is derived from the first tab belonging to each group because Chrome does not expose a standalone group index on the group object.

## Restore

Restore creates new windows and keeps the current workspace intact.

A blank bootstrap tab makes window creation independent from imported URLs. Restorable tabs are then created one by one. Rejected tabs become warnings instead of aborting the whole restore.

Groups, pinning, active selection and final window state are applied after tab creation.

## Security

Imported data is validated before browser APIs are called.

`javascript:`, `data:`, `blob:`, `filesystem:`, `devtools:` and `chrome-extension:` URLs are not attempted during restore.

The extension has no content scripts and no network-facing host permissions.
