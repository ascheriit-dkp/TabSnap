import { describe, expect, it, vi } from 'vitest';

import {
  CompanionClient,
  MAX_COMPANION_SNAPSHOT_BYTES,
  parseCompanionPairingCode,
} from './companion.js';

const TOKEN = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const PAIRING = `tabsnap-companion:v1:43123:${TOKEN}`;

function jsonResponse(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('parseCompanionPairingCode', () => {
  it('accepts only a v1 loopback pairing code', () => {
    expect(parseCompanionPairingCode(PAIRING)).toEqual({
      endpoint: 'http://127.0.0.1:43123',
      token: TOKEN,
    });
  });

  it.each([
    `tabsnap-companion:v2:43123:${TOKEN}`,
    `tabsnap-companion:v1:0:${TOKEN}`,
    `tabsnap-companion:v1:65536:${TOKEN}`,
    `tabsnap-companion:v1:43123:${'g'.repeat(64)}`,
    `tabsnap-companion:v1:43123:127.0.0.1:${TOKEN}`,
    `https://example.com:${TOKEN}`,
  ])('rejects invalid pairing code %s', (value) => {
    expect(() => parseCompanionPairingCode(value)).toThrow();
  });
});

describe('CompanionClient', () => {
  it('authenticates status requests and validates protocol version', async () => {
    const fetchImpl = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) =>
      jsonResponse({
        protocolVersion: 1,
        transport: 'loopback-http',
        authentication: 'session-bearer',
      }),
    );
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.status()).resolves.toEqual({
      protocolVersion: 1,
      transport: 'loopback-http',
      authentication: 'session-bearer',
    });

    const [url, init] = fetchImpl.mock.calls[0] ?? [];
    expect(url).toBe('http://127.0.0.1:43123/v1/status');
    expect(new Headers(init?.headers).get('Authorization')).toBe(`Bearer ${TOKEN}`);
    expect(init?.credentials).toBe('omit');
    expect(init?.redirect).toBe('error');
    expect(init?.cache).toBe('no-store');
  });

  it('rejects an incompatible status version', async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse({
        protocolVersion: 2,
        transport: 'loopback-http',
        authentication: 'session-bearer',
      }),
    );
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.status()).rejects.toThrow('incompatible');
  });

  it('lists validated library entries', async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse({
        protocolVersion: 1,
        snapshots: [
          { name: 'alpha.tabsnap', size: 12 },
          { name: 'work.tabsnap', size: 42 },
        ],
      }),
    );
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.listSnapshots()).resolves.toEqual([
      { name: 'alpha.tabsnap', size: 12 },
      { name: 'work.tabsnap', size: 42 },
    ]);
  });

  it('sends opaque encrypted bytes unchanged', async () => {
    const fetchImpl = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      expect(init?.method).toBe('POST');
      expect(new Headers(init?.headers).get('Content-Type')).toBe('application/octet-stream');
      expect(new Headers(init?.headers).get('X-TabSnap-Name')).toBe('Work Session');
      const body = init?.body;
      expect(body).toBeInstanceOf(ArrayBuffer);
      expect(Array.from(new Uint8Array(body as ArrayBuffer))).toEqual([0, 1, 2, 255]);
      return jsonResponse({ name: 'Work Session.tabsnap', size: 4 }, 201);
    });
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(
      client.storeSnapshot('Work Session', new Uint8Array([0, 1, 2, 255])),
    ).resolves.toEqual({
      name: 'Work Session.tabsnap',
      size: 4,
    });
  });

  it('loads opaque bytes unchanged', async () => {
    const expected = new Uint8Array([9, 8, 7, 0, 255]);
    const fetchImpl = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      expect(new Headers(init?.headers).get('X-TabSnap-Name')).toBe('work.tabsnap');
      return new Response(expected, {
        status: 200,
        headers: {
          'Content-Type': 'application/octet-stream',
          'Content-Length': String(expected.byteLength),
        },
      });
    });
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.loadSnapshot('work.tabsnap')).resolves.toEqual(expected);
  });

  it('rejects oversized advertised snapshots before reading the body', async () => {
    const fetchImpl = vi.fn(
      async () =>
        new Response(new Uint8Array([1]), {
          status: 200,
          headers: {
            'Content-Type': 'application/octet-stream',
            'Content-Length': String(MAX_COMPANION_SNAPSHOT_BYTES + 1),
          },
        }),
    );
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.loadSnapshot('huge.tabsnap')).rejects.toThrow('size limit');
  });

  it('surfaces companion JSON errors without exposing the session token', async () => {
    const fetchImpl = vi.fn(async () => jsonResponse({ error: 'Snapshot not found.' }, 404));
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.loadSnapshot('missing.tabsnap')).rejects.toThrow(
      'Companion error (404): Snapshot not found.',
    );
    await expect(client.loadSnapshot('missing.tabsnap')).rejects.not.toThrow(TOKEN);
  });
});
