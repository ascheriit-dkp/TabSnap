import initializeArgon2 from '@phi-ag/argon2/node';
import type Argon2 from '@phi-ag/argon2';
import type { TabSnapSnapshot } from '@tabsnap/schema';
import { beforeAll, describe, expect, it } from 'vitest';

import {
  decryptSnapshot,
  encryptSnapshot,
  exportSnapshotString,
  fromBase64Url,
  importSnapshotString,
  STRING_PREFIX,
  toBase64Url,
} from './index.js';

let argon2: Argon2;

beforeAll(async () => {
  argon2 = await initializeArgon2();
});

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

describe('Base64URL', () => {
  it('round-trips bytes', () => {
    const bytes = crypto.getRandomValues(new Uint8Array(257));
    expect(fromBase64Url(toBase64Url(bytes))).toEqual(bytes);
  });

  it('rejects invalid characters', () => {
    expect(() => fromBase64Url('nope+')).toThrow('Invalid Base64URL data.');
  });
});

describe('encrypted snapshots', () => {
  it('round-trips a binary envelope', async () => {
    const encrypted = await encryptSnapshot(snapshot(), 'correct horse battery staple', argon2);
    expect(await decryptSnapshot(encrypted, 'correct horse battery staple', argon2)).toEqual(
      snapshot(),
    );
  });

  it('round-trips a copyable string', async () => {
    const value = await exportSnapshotString(snapshot(), 'correct horse battery staple', argon2);
    expect(value.startsWith(STRING_PREFIX)).toBe(true);
    expect(await importSnapshotString(value, 'correct horse battery staple', argon2)).toEqual(
      snapshot(),
    );
  });

  it('rejects a wrong password without decrypting', async () => {
    const encrypted = await encryptSnapshot(snapshot(), 'correct horse battery staple', argon2);
    await expect(decryptSnapshot(encrypted, 'definitely the wrong password', argon2)).rejects.toThrow(
      'Unable to decrypt snapshot.',
    );
  });

  it('rejects ciphertext tampering', async () => {
    const encrypted = await encryptSnapshot(snapshot(), 'correct horse battery staple', argon2);
    encrypted[encrypted.length - 1] ^= 1;
    await expect(decryptSnapshot(encrypted, 'correct horse battery staple', argon2)).rejects.toThrow(
      'Unable to decrypt snapshot.',
    );
  });

  it('rejects weak passwords', async () => {
    await expect(encryptSnapshot(snapshot(), 'short', argon2)).rejects.toThrow(
      'Password must be at least 8 characters.',
    );
  });
});
