import { describe, expect, it } from 'vitest';

import type { TabSnapSnapshot } from '@tabsnap/schema';

import {
  MAX_DECOMPRESSED_SNAPSHOT_BYTES,
  compressSnapshot,
  decodeSnapshot,
  decompressSnapshot,
  deserializeSnapshot,
  encodeSnapshot,
  serializeSnapshot,
} from './index.js';

function snapshot(): TabSnapSnapshot {
  return {
    format: 'tabsnap',
    formatVersion: 1,
    createdAt: '2026-09-04T14:00:00+02:00',
    source: { browser: 'chrome', platform: 'windows' },
    windows: [
      {
        order: 0,
        state: 'normal',
        focused: true,
        groups: [],
        tabs: [
          {
            order: 0,
            url: 'https://example.com/?q=héllo',
            title: 'Example 👀',
            pinned: false,
            active: true,
          },
        ],
      },
    ],
  };
}

describe('snapshot serialization', () => {
  it('round-trips valid snapshots', () => {
    expect(deserializeSnapshot(serializeSnapshot(snapshot()))).toEqual(snapshot());
  });

  it('rejects invalid decoded data', () => {
    const invalid = new TextEncoder().encode('{"format":"nope"}');
    expect(() => deserializeSnapshot(invalid)).toThrow();
  });

  it('rejects invalid UTF-8', () => {
    expect(() => deserializeSnapshot(new Uint8Array([0xff]))).toThrow();
  });

  it('rejects oversized decoded input before parsing', () => {
    expect(() => deserializeSnapshot(new Uint8Array(MAX_DECOMPRESSED_SNAPSHOT_BYTES + 1))).toThrow(
      'Snapshot exceeds the maximum decoded size.',
    );
  });
});

describe('snapshot compression', () => {
  it('round-trips bytes through gzip', async () => {
    const input = new TextEncoder().encode('hello '.repeat(1000));
    const compressed = await compressSnapshot(input);
    expect(await decompressSnapshot(compressed)).toEqual(input);
  });

  it('round-trips a full snapshot', async () => {
    expect(await decodeSnapshot(await encodeSnapshot(snapshot()))).toEqual(snapshot());
  });

  it('rejects invalid gzip input', async () => {
    await expect(decompressSnapshot(new Uint8Array([1, 2, 3]))).rejects.toThrow();
  });
});
