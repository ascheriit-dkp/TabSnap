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

`0.1.0-beta.1` is the Level 1 Chrome beta. It captures, encrypts, exports, imports and restores browser workspaces locally.

Clicking the TabSnap toolbar action opens a persistent extension page, so capture, Argon2id encryption, decryption and restore are no longer tied to the lifetime of a browser popup.

See the [roadmap](./ROADMAP.md), [Chrome guide](./docs/guide/chrome-extension.md) and [product spec](./docs/product-spec.md).

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
