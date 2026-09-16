# TabSnap

Move a browser workspace between machines.

No account. No backend. No sync service.

Capture windows and tabs, encrypt the snapshot, move it yourself, restore it somewhere else.

## Rules

- snapshot data stays local in extension-only mode
- no telemetry
- no analytics
- no page content, cookies or history
- no password storage
- restore is non-destructive by default
- optional companion traffic is encrypted and loopback-only

## Status

`v0.3.0-alpha.3` is the current Level 3 prerelease.

Chrome, Microsoft Edge and Firefox can capture and restore browser workspaces using the same `.tabsnap` format. Chrome ↔ Edge restore is supported through the shared Chromium implementation. Chromium ↔ Firefox restore is best-effort and reports known browser-specific tabs before restore.

The optional portable Windows companion stores opaque encrypted snapshots locally or on removable storage. Extension-only mode does not require it.

Store submission preparation is in progress. See the [roadmap](./ROADMAP.md), [guides](./docs/guide/), [privacy policy](./docs/privacy.md) and [product spec](./docs/product-spec.md).

## Development

```bash
pnpm install
pnpm check
pnpm docs:dev
```

Build Chromium:

```bash
pnpm extension:build
```

Build and validate Firefox:

```bash
pnpm extension:build
pnpm extension:firefox-build
pnpm extension:firefox-audit
pnpm extension:firefox-lint
```

Node 24+.

## License

MIT.
