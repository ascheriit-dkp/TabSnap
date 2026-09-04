# `.tabsnap` format

The snapshot model is browser-agnostic.

Current version: `1`.

This is the plaintext model before serialization, compression and encryption. A `.tabsnap` file will not store this JSON in plaintext once the crypto layer lands.

## Root

```json
{
  "format": "tabsnap",
  "formatVersion": 1,
  "createdAt": "2026-09-04T14:00:00+02:00",
  "source": {
    "browser": "chrome",
    "browserVersion": "140.0.0.0",
    "platform": "windows"
  },
  "windows": []
}
```

`browser` is one of `chrome`, `edge`, `firefox`.

`platform` is one of `windows`, `macos`, `linux`.

## Window

```json
{
  "order": 0,
  "state": "normal",
  "focused": true,
  "bounds": {
    "left": 0,
    "top": 0,
    "width": 1920,
    "height": 1080
  },
  "groups": [],
  "tabs": []
}
```

Window state is `normal`, `maximized`, `minimized` or `fullscreen`.

Bounds are optional. Restore code must handle different monitor layouts.

## Tab

```json
{
  "order": 0,
  "url": "https://example.com/",
  "title": "Example",
  "pinned": false,
  "active": true,
  "groupId": "research"
}
```

`title` and `groupId` are optional.

A window has exactly one active tab.

## Group

```json
{
  "id": "research",
  "order": 0,
  "title": "Research",
  "color": "blue",
  "collapsed": false
}
```

`title` and `color` are optional.

Group IDs and orders are unique inside a window.

## Validation

Unknown fields are rejected.

Orders must be unique in their scope.

Tabs cannot reference missing groups.

Input sizes are bounded. Imported snapshots are untrusted input.

## Versioning

Version `1` is strict. A future format change gets a new version and an explicit migration path. Old snapshots do not silently change meaning.
