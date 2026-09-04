# TabSnap

Move a browser workspace between machines.

No account. No backend. No sync service.

Capture windows and tabs, encrypt the snapshot, move it yourself, restore it somewhere else.

## Rules

- snapshot data stays local
- no telemetry
- no analytics
- no page content, cookies or history
- no password storage
- restore is non-destructive by default

## Status

The browser-agnostic snapshot, compression and encryption core is implemented. The Chrome Manifest V3 client is the current Level 1 target.

See the [roadmap](./ROADMAP.md) and [product spec](./docs/product-spec.md).

## Development

```bash
pnpm install
pnpm check
pnpm docs:dev
```

Build the unpacked Chrome extension with:

```bash
pnpm extension:build
```

Then load `apps/chrome-extension/dist` from `chrome://extensions` with Developer mode enabled.

Node 24+.

## License

MIT.
