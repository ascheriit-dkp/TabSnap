import type { TabSnapSnapshot } from '@tabsnap/schema';
import { describe, expect, it } from 'vitest';

import {
  decryptSnapshot,
  encryptSnapshot,
  exportSnapshotString,
  fromBase64Url,
  importSnapshotString,
  STRING_PREFIX,
  toBase64Url,
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

function tamperHeaderNumber(encrypted: Uint8Array, from: string, to: string): Uint8Array {
  if (from.length !== to.length) throw new Error('Header replacement must preserve byte length.');

  const copy = encrypted.slice();
  const headerLength = new DataView(copy.buffer, copy.byteOffset + 9, 4).getUint32(0, false);
  const headerStart = 13;
  const headerEnd = headerStart + headerLength;
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  const header = decoder.decode(copy.subarray(headerStart, headerEnd));
  const replaced = header.replace(from, to);

  if (replaced === header) throw new Error(`Could not find ${from} in encrypted header.`);
  const bytes = encoder.encode(replaced);
  if (bytes.byteLength !== headerLength) throw new Error('Header replacement changed byte length.');
  copy.set(bytes, headerStart);
  return copy;
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
    const encrypted = await encryptSnapshot(snapshot(), 'correct horse battery staple');
    expect(await decryptSnapshot(encrypted, 'correct horse battery staple')).toEqual(snapshot());
  });

  it('round-trips a copyable string', async () => {
    const value = await exportSnapshotString(snapshot(), 'correct horse battery staple');
    expect(value.startsWith(STRING_PREFIX)).toBe(true);
    expect(await importSnapshotString(value, 'correct horse battery staple')).toEqual(snapshot());
  });

  it('round-trips a Unicode password as UTF-8 bytes', async () => {
    const password = 'clé très sûre 🔐 你好';
    const encrypted = await encryptSnapshot(snapshot(), password);
    expect(await decryptSnapshot(encrypted, password)).toEqual(snapshot());
  });

  it('rejects a wrong password without decrypting', async () => {
    const encrypted = await encryptSnapshot(snapshot(), 'correct horse battery staple');
    await expect(decryptSnapshot(encrypted, 'definitely the wrong password')).rejects.toThrow(
      'Unable to decrypt snapshot.',
    );
  });

  it('rejects ciphertext tampering', async () => {
    const encrypted = await encryptSnapshot(snapshot(), 'correct horse battery staple');
    const lastIndex = encrypted.length - 1;
    encrypted[lastIndex] = encrypted[lastIndex]! ^ 1;
    await expect(decryptSnapshot(encrypted, 'correct horse battery staple')).rejects.toThrow(
      'Unable to decrypt snapshot.',
    );
  });

  it('rejects attacker-controlled Argon2 memory before running a different KDF profile', async () => {
    const encrypted = await encryptSnapshot(snapshot(), 'correct horse battery staple');
    const tampered = tamperHeaderNumber(encrypted, '"memoryKiB":65536', '"memoryKiB":65537');
    await expect(decryptSnapshot(tampered, 'correct horse battery staple')).rejects.toThrow();
  });

  it('rejects attacker-controlled Argon2 iterations before running a different KDF profile', async () => {
    const encrypted = await encryptSnapshot(snapshot(), 'correct horse battery staple');
    const tampered = tamperHeaderNumber(encrypted, '"iterations":3', '"iterations":9');
    await expect(decryptSnapshot(tampered, 'correct horse battery staple')).rejects.toThrow();
  });

  it('rejects weak passwords', async () => {
    await expect(encryptSnapshot(snapshot(), 'short')).rejects.toThrow(
      'Password must be at least 8 characters.',
    );
  });
});
