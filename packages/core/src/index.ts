import { parseTabSnapSnapshot, type TabSnapSnapshot } from '@tabsnap/schema';

export const MAX_DECOMPRESSED_SNAPSHOT_BYTES = 64 * 1024 * 1024;

const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

export function serializeSnapshot(snapshot: TabSnapSnapshot): Uint8Array {
  const validated = parseTabSnapSnapshot(snapshot);
  return encoder.encode(JSON.stringify(validated));
}

export function deserializeSnapshot(bytes: Uint8Array): TabSnapSnapshot {
  if (bytes.byteLength > MAX_DECOMPRESSED_SNAPSHOT_BYTES) {
    throw new Error('Snapshot exceeds the maximum decoded size.');
  }

  const text = decoder.decode(bytes);
  const input: unknown = JSON.parse(text);
  return parseTabSnapSnapshot(input);
}

async function readStreamWithLimit(
  stream: ReadableStream<Uint8Array>,
  maxBytes: number,
): Promise<Uint8Array> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (value === undefined) continue;

      total += value.byteLength;
      if (total > maxBytes) {
        await reader.cancel('Snapshot exceeds the maximum decoded size.');
        throw new Error('Snapshot exceeds the maximum decoded size.');
      }

      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }

  const output = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    output.set(chunk, offset);
    offset += chunk.byteLength;
  }

  return output;
}

export async function compressSnapshot(bytes: Uint8Array): Promise<Uint8Array> {
  const stream = new Blob([bytes])
    .stream()
    .pipeThrough(new CompressionStream('gzip')) as ReadableStream<Uint8Array>;

  return readStreamWithLimit(stream, MAX_DECOMPRESSED_SNAPSHOT_BYTES);
}

export async function decompressSnapshot(bytes: Uint8Array): Promise<Uint8Array> {
  const stream = new Blob([bytes])
    .stream()
    .pipeThrough(new DecompressionStream('gzip')) as ReadableStream<Uint8Array>;

  return readStreamWithLimit(stream, MAX_DECOMPRESSED_SNAPSHOT_BYTES);
}

export async function encodeSnapshot(snapshot: TabSnapSnapshot): Promise<Uint8Array> {
  return compressSnapshot(serializeSnapshot(snapshot));
}

export async function decodeSnapshot(bytes: Uint8Array): Promise<TabSnapSnapshot> {
  return deserializeSnapshot(await decompressSnapshot(bytes));
}
