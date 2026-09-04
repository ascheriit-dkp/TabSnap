import { argon2id } from 'hash-wasm';
import { decodeSnapshot, encodeSnapshot, MAX_COMPRESSED_SNAPSHOT_BYTES } from '@tabsnap/core';
import type { TabSnapSnapshot } from '@tabsnap/schema';
import { z } from 'zod';

export const ENVELOPE_VERSION = 1 as const;
export const STRING_PREFIX = 'tabsnap:v1:' as const;
export const MIN_PASSWORD_LENGTH = 8;

const MAGIC = new TextEncoder().encode('TABSNAP\0');
const HEADER_LENGTH_BYTES = 4;
const PREFIX_BYTES = MAGIC.byteLength + 1 + HEADER_LENGTH_BYTES;
const MAX_HEADER_BYTES = 4 * 1024;
const MAX_ENCRYPTED_BYTES = MAX_COMPRESSED_SNAPSHOT_BYTES + MAX_HEADER_BYTES + PREFIX_BYTES + 16;

const ARGON2_MEMORY_KIB = 65_536;
const ARGON2_ITERATIONS = 3;
const ARGON2_PARALLELISM = 4;
const ARGON2_SALT_BYTES = 16;
const AES_KEY_BYTES = 32;
const AES_GCM_IV_BYTES = 12;

const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

const headerSchema = z
  .object({
    kdf: z
      .object({
        name: z.literal('argon2id'),
        version: z.literal(19),
        memoryKiB: z
          .number()
          .int()
          .min(32 * 1024)
          .max(256 * 1024),
        iterations: z.number().int().min(1).max(10),
        parallelism: z.number().int().min(1).max(8),
        salt: z.string().min(1).max(128),
      })
      .strict(),
    cipher: z
      .object({
        name: z.literal('aes-256-gcm'),
        iv: z.string().min(1).max(128),
      })
      .strict(),
    compression: z.literal('gzip'),
  })
  .strict();

type EnvelopeHeader = z.infer<typeof headerSchema>;

function assertPassword(password: string): void {
  if (password.length < MIN_PASSWORD_LENGTH) {
    throw new Error(`Password must be at least ${MIN_PASSWORD_LENGTH} characters.`);
  }
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const length = parts.reduce((total, part) => total + part.byteLength, 0);
  const output = new Uint8Array(length);
  let offset = 0;

  for (const part of parts) {
    output.set(part, offset);
    offset += part.byteLength;
  }

  return output;
}

function equalBytes(left: Uint8Array, right: Uint8Array): boolean {
  if (left.byteLength !== right.byteLength) return false;
  let difference = 0;
  for (let index = 0; index < left.byteLength; index += 1) {
    difference |= left[index]! ^ right[index]!;
  }
  return difference === 0;
}

export function toBase64Url(bytes: Uint8Array): string {
  const chunkSize = 32_768;
  let binary = '';

  for (let offset = 0; offset < bytes.byteLength; offset += chunkSize) {
    const chunk = bytes.subarray(offset, Math.min(offset + chunkSize, bytes.byteLength));
    for (const byte of chunk) binary += String.fromCharCode(byte);
  }

  return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/u, '');
}

export function fromBase64Url(value: string): Uint8Array {
  if (!/^[A-Za-z0-9_-]*$/u.test(value)) throw new Error('Invalid Base64URL data.');

  const padding = '='.repeat((4 - (value.length % 4)) % 4);
  const base64 = value.replaceAll('-', '+').replaceAll('_', '/') + padding;
  const binary = atob(base64);
  const output = new Uint8Array(binary.length);

  for (let index = 0; index < binary.length; index += 1) {
    output[index] = binary.charCodeAt(index);
  }

  return output;
}

function encodeHeader(header: EnvelopeHeader): Uint8Array {
  return encoder.encode(JSON.stringify(header));
}

function buildAuthenticatedHeader(header: EnvelopeHeader): Uint8Array {
  const headerBytes = encodeHeader(header);
  if (headerBytes.byteLength > MAX_HEADER_BYTES) throw new Error('Envelope header is too large.');

  const prefix = new Uint8Array(PREFIX_BYTES);
  prefix.set(MAGIC, 0);
  prefix[MAGIC.byteLength] = ENVELOPE_VERSION;
  new DataView(prefix.buffer).setUint32(MAGIC.byteLength + 1, headerBytes.byteLength, false);

  return concatBytes(prefix, headerBytes);
}

function parseEnvelope(bytes: Uint8Array): {
  header: EnvelopeHeader;
  authenticatedHeader: Uint8Array;
  ciphertext: Uint8Array;
} {
  if (bytes.byteLength > MAX_ENCRYPTED_BYTES) throw new Error('Encrypted snapshot is too large.');
  if (bytes.byteLength < PREFIX_BYTES + 16) throw new Error('Invalid encrypted snapshot.');

  const magic = bytes.subarray(0, MAGIC.byteLength);
  if (!equalBytes(magic, MAGIC)) throw new Error('Invalid encrypted snapshot.');

  const version = bytes[MAGIC.byteLength];
  if (version !== ENVELOPE_VERSION) throw new Error('Unsupported envelope version.');

  const headerLength = new DataView(
    bytes.buffer,
    bytes.byteOffset + MAGIC.byteLength + 1,
    HEADER_LENGTH_BYTES,
  ).getUint32(0, false);

  if (headerLength === 0 || headerLength > MAX_HEADER_BYTES) {
    throw new Error('Invalid encrypted snapshot header.');
  }

  const headerEnd = PREFIX_BYTES + headerLength;
  if (headerEnd + 16 > bytes.byteLength) throw new Error('Invalid encrypted snapshot.');

  const headerText = decoder.decode(bytes.subarray(PREFIX_BYTES, headerEnd));
  const input: unknown = JSON.parse(headerText);
  const header = headerSchema.parse(input);

  const salt = fromBase64Url(header.kdf.salt);
  const iv = fromBase64Url(header.cipher.iv);
  if (salt.byteLength !== ARGON2_SALT_BYTES) throw new Error('Invalid Argon2 salt.');
  if (iv.byteLength !== AES_GCM_IV_BYTES) throw new Error('Invalid AES-GCM IV.');

  return {
    header,
    authenticatedHeader: bytes.slice(0, headerEnd),
    ciphertext: bytes.slice(headerEnd),
  };
}

async function deriveKeyBytes(password: string, header: EnvelopeHeader): Promise<Uint8Array> {
  const key = await argon2id({
    password: encoder.encode(password),
    salt: fromBase64Url(header.kdf.salt),
    iterations: header.kdf.iterations,
    parallelism: header.kdf.parallelism,
    memorySize: header.kdf.memoryKiB,
    hashLength: AES_KEY_BYTES,
    outputType: 'binary',
  });

  return key;
}

async function importAesKey(keyBytes: Uint8Array): Promise<CryptoKey> {
  return crypto.subtle.importKey('raw', toArrayBuffer(keyBytes), 'AES-GCM', false, [
    'encrypt',
    'decrypt',
  ]);
}

export async function encryptSnapshot(
  snapshot: TabSnapSnapshot,
  password: string,
): Promise<Uint8Array> {
  assertPassword(password);

  const salt = crypto.getRandomValues(new Uint8Array(ARGON2_SALT_BYTES));
  const iv = crypto.getRandomValues(new Uint8Array(AES_GCM_IV_BYTES));
  const header: EnvelopeHeader = {
    kdf: {
      name: 'argon2id',
      version: 19,
      memoryKiB: ARGON2_MEMORY_KIB,
      iterations: ARGON2_ITERATIONS,
      parallelism: ARGON2_PARALLELISM,
      salt: toBase64Url(salt),
    },
    cipher: {
      name: 'aes-256-gcm',
      iv: toBase64Url(iv),
    },
    compression: 'gzip',
  };

  const authenticatedHeader = buildAuthenticatedHeader(header);
  const key = await importAesKey(await deriveKeyBytes(password, header));
  const plaintext = await encodeSnapshot(snapshot);
  const ciphertext = await crypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: toArrayBuffer(iv),
      additionalData: toArrayBuffer(authenticatedHeader),
      tagLength: 128,
    },
    key,
    toArrayBuffer(plaintext),
  );

  return concatBytes(authenticatedHeader, new Uint8Array(ciphertext));
}

export async function decryptSnapshot(
  bytes: Uint8Array,
  password: string,
): Promise<TabSnapSnapshot> {
  assertPassword(password);
  const { header, authenticatedHeader, ciphertext } = parseEnvelope(bytes);
  const key = await importAesKey(await deriveKeyBytes(password, header));

  let plaintext: ArrayBuffer;
  try {
    plaintext = await crypto.subtle.decrypt(
      {
        name: 'AES-GCM',
        iv: toArrayBuffer(fromBase64Url(header.cipher.iv)),
        additionalData: toArrayBuffer(authenticatedHeader),
        tagLength: 128,
      },
      key,
      toArrayBuffer(ciphertext),
    );
  } catch {
    throw new Error('Unable to decrypt snapshot.');
  }

  return decodeSnapshot(new Uint8Array(plaintext));
}

export async function exportSnapshotString(
  snapshot: TabSnapSnapshot,
  password: string,
): Promise<string> {
  return STRING_PREFIX + toBase64Url(await encryptSnapshot(snapshot, password));
}

export async function importSnapshotString(
  value: string,
  password: string,
): Promise<TabSnapSnapshot> {
  const trimmed = value.trim();
  if (!trimmed.startsWith(STRING_PREFIX)) throw new Error('Invalid TabSnap string.');
  return decryptSnapshot(fromBase64Url(trimmed.slice(STRING_PREFIX.length)), password);
}
