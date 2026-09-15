# Changelog

All notable changes will be documented here.

The project uses Semantic Versioning once releases start.

## Unreleased

## 0.1.0-alpha.1 - 2026-09-15

First public alpha of the Level 1 Chrome extension.

### Added

- browser-agnostic `.tabsnap` schema with runtime validation
- gzip serialization and bounded decompression
- Argon2id password derivation and AES-256-GCM authenticated encryption
- copyable `tabsnap:v1:` encrypted snapshots
- encrypted `.tabsnap` file export/import
- Chrome Manifest V3 capture of windows, tabs, pinning, active tabs and tab groups
- non-destructive restore into new windows
- window geometry and state restore when available
- preview, warnings and per-tab restore error handling
- static extension audit for permissions, remote resources and JavaScript network primitives
- real Chromium integration coverage for capture and restore

### Security and privacy

- no account, backend, telemetry or analytics
- no host permissions
- no page-content scripts
- no cookies or history access
- only `tabs` and `tabGroups` extension permissions
- unsupported extension/internal URLs are skipped during restore rather than executed

### Known limitation

Long operations currently run in the extension popup. Keep the popup open until capture, encryption, decryption or restore completes.
