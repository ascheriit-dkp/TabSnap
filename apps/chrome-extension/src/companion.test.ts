import { describe, expect, it, vi } from 'vitest';

import {
  CompanionClient,
  MAX_COMPANION_SNAPSHOT_BYTES,
  createCompanionBrowserInstanceId,
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
  it('binds the default browser fetch to globalThis', async () => {
    const fetchImpl = vi.fn(function (
      this: unknown,
      input: RequestInfo | URL,
      init?: RequestInit,
    ): Promise<Response> {
      expect(this).toBe(globalThis);
      expect(input).toBe('http://127.0.0.1:43123/v1/status');
      expect(new Headers(init?.headers).get('Authorization')).toBe(`Bearer ${TOKEN}`);
      return Promise.resolve(
        jsonResponse({
          protocolVersion: 1,
          transport: 'loopback-http',
          authentication: 'session-bearer',
        }),
      );
    });
    vi.stubGlobal('fetch', fetchImpl);

    try {
      const client = new CompanionClient(parseCompanionPairingCode(PAIRING));
      await expect(client.status()).resolves.toMatchObject({ protocolVersion: 1 });
      expect(fetchImpl).toHaveBeenCalledTimes(1);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('authenticates status requests and validates protocol version', async () => {
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      expect(input).toBe('http://127.0.0.1:43123/v1/status');
      expect(new Headers(init?.headers).get('Authorization')).toBe(`Bearer ${TOKEN}`);
      expect(init?.credentials).toBe('omit');
      expect(init?.redirect).toBe('error');
      expect(init?.cache).toBe('no-store');
      return jsonResponse({
        protocolVersion: 1,
        transport: 'loopback-http',
        authentication: 'session-bearer',
      });
    });
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.status()).resolves.toEqual({
      protocolVersion: 1,
      transport: 'loopback-http',
      authentication: 'session-bearer',
    });
    expect(fetchImpl).toHaveBeenCalledTimes(1);
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

  it('accepts and validates coordination status', async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse({
        protocolVersion: 1,
        transport: 'loopback-http',
        authentication: 'session-bearer',
        coordinationVersion: 1,
        browserLeaseSeconds: 30,
        captureVersion: 1,
        restoreVersion: 1,
      }),
    );
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.status()).resolves.toMatchObject({
      coordinationVersion: 1,
      browserLeaseSeconds: 30,
      captureVersion: 1,
      restoreVersion: 1,
    });
  });

  it('polls capture assignments and accepts an empty queue', async () => {
    const fetchImpl = vi
      .fn()
      .mockResolvedValueOnce(
        jsonResponse({
          protocolVersion: 1,
          coordinationVersion: 1,
          captureVersion: 1,
          jobId: '11111111111111111111111111111111',
        }),
      )
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.pollCapture('0123456789abcdef0123456789abcdef')).resolves.toEqual({
      jobId: '11111111111111111111111111111111',
    });
    await expect(client.pollCapture('0123456789abcdef0123456789abcdef')).resolves.toBeUndefined();

    for (const call of fetchImpl.mock.calls) {
      expect(String(call[0])).toMatch(/\/v1\/browser\/capture\/poll$/u);
      expect(call[1]?.method).toBe('POST');
      expect(call[1]?.body).toBe('tabsnap-capture:v1\n0123456789abcdef0123456789abcdef');
    }
  });

  it('submits opaque coordinated capture bytes and bounded failure codes', async () => {
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/v1/browser/capture/result')) {
        const headers = new Headers(init?.headers);
        expect(headers.get('Content-Type')).toBe('application/octet-stream');
        expect(headers.get('X-TabSnap-Instance')).toBe('0123456789abcdef0123456789abcdef');
        expect(headers.get('X-TabSnap-Job')).toBe('11111111111111111111111111111111');
        expect(Array.from(new Uint8Array(init?.body as ArrayBuffer))).toEqual([1, 2, 3, 255]);
      } else {
        expect(url).toMatch(/\/v1\/browser\/capture\/failure$/u);
        expect(init?.body).toBe(
          'tabsnap-capture:v1\n0123456789abcdef0123456789abcdef\n11111111111111111111111111111111\npassword-required',
        );
      }
      return new Response(null, { status: 204 });
    });
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await client.submitCaptureResult(
      '0123456789abcdef0123456789abcdef',
      '11111111111111111111111111111111',
      new Uint8Array([1, 2, 3, 255]),
    );
    await client.submitCaptureFailure(
      '0123456789abcdef0123456789abcdef',
      '11111111111111111111111111111111',
      'password-required',
    );
    expect(fetchImpl).toHaveBeenCalledTimes(2);
  });

  it('polls opaque coordinated restore payloads and accepts an empty queue', async () => {
    const encrypted = new Uint8Array([9, 8, 7, 0, 255]);
    const fetchImpl = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(encrypted, {
          status: 200,
          headers: {
            'Content-Type': 'application/octet-stream',
            'Content-Length': String(encrypted.byteLength),
            'X-TabSnap-Restore-Job': '22222222222222222222222222222222',
            'X-TabSnap-Source-Instance': 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            'X-TabSnap-Source-Browser': 'edge',
          },
        }),
      )
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.pollRestore('0123456789abcdef0123456789abcdef')).resolves.toEqual({
      jobId: '22222222222222222222222222222222',
      sourceInstanceId: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      sourceBrowser: 'edge',
      encrypted,
    });
    await expect(client.pollRestore('0123456789abcdef0123456789abcdef')).resolves.toBeUndefined();

    for (const call of fetchImpl.mock.calls) {
      expect(String(call[0])).toMatch(/\/v1\/browser\/restore\/poll$/u);
      expect(call[1]?.method).toBe('POST');
      expect(call[1]?.body).toBe('tabsnap-restore:v1\n0123456789abcdef0123456789abcdef');
    }
  });

  it('submits coordinated restore success and bounded failure codes', async () => {
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/v1/browser/restore/result')) {
        expect(init?.body).toBe(
          'tabsnap-restore:v1\n0123456789abcdef0123456789abcdef\n22222222222222222222222222222222',
        );
      } else {
        expect(url).toMatch(/\/v1\/browser\/restore\/failure$/u);
        expect(init?.body).toBe(
          'tabsnap-restore:v1\n0123456789abcdef0123456789abcdef\n22222222222222222222222222222222\ndecrypt-failed',
        );
      }
      return new Response(null, { status: 204 });
    });
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await client.submitRestoreSuccess(
      '0123456789abcdef0123456789abcdef',
      '22222222222222222222222222222222',
    );
    await client.submitRestoreFailure(
      '0123456789abcdef0123456789abcdef',
      '22222222222222222222222222222222',
      'decrypt-failed',
    );
    expect(fetchImpl).toHaveBeenCalledTimes(2);
  });

  it('rejects oversized or malformed coordinated restore assignments', async () => {
    const oversizedFetch = vi.fn(async () =>
      new Response(new Uint8Array([1]), {
        status: 200,
        headers: {
          'Content-Type': 'application/octet-stream',
          'Content-Length': String(MAX_COMPANION_SNAPSHOT_BYTES + 1),
          'X-TabSnap-Restore-Job': '22222222222222222222222222222222',
          'X-TabSnap-Source-Instance': 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          'X-TabSnap-Source-Browser': 'chrome',
        },
      }),
    );
    const oversizedClient = new CompanionClient(parseCompanionPairingCode(PAIRING), {
      fetchImpl: oversizedFetch,
    });
    await expect(
      oversizedClient.pollRestore('0123456789abcdef0123456789abcdef'),
    ).rejects.toThrow('size limit');

    const malformedFetch = vi.fn(async () =>
      new Response(new Uint8Array([1, 2, 3]), {
        status: 200,
        headers: {
          'Content-Type': 'application/octet-stream',
          'X-TabSnap-Restore-Job': '22222222222222222222222222222222',
          'X-TabSnap-Source-Instance': 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          'X-TabSnap-Source-Browser': 'safari',
        },
      }),
    );
    const malformedClient = new CompanionClient(parseCompanionPairingCode(PAIRING), {
      fetchImpl: malformedFetch,
    });
    await expect(
      malformedClient.pollRestore('0123456789abcdef0123456789abcdef'),
    ).rejects.toThrow('invalid restore assignment');
  });

  it('registers browser presence and sends heartbeats', async () => {
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      expect(init?.method).toBe('POST');
      expect(new Headers(init?.headers).get('Content-Type')).toContain('text/plain');
      if (url.endsWith('/v1/browser/register')) {
        expect(init?.body).toBe(
          'tabsnap-browser:v1\n0123456789abcdef0123456789abcdef\nfirefox\n143.0\ncapture,restore',
        );
      } else {
        expect(url).toMatch(/\/v1\/browser\/heartbeat$/u);
        expect(init?.body).toBe('tabsnap-browser:v1\n0123456789abcdef0123456789abcdef');
      }
      return new Response(null, { status: 204 });
    });
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await client.registerBrowser({
      instanceId: '0123456789abcdef0123456789abcdef',
      browser: 'firefox',
      version: '143.0',
      capabilities: ['capture', 'restore'],
    });
    await client.heartbeatBrowser('0123456789abcdef0123456789abcdef');
    expect(fetchImpl).toHaveBeenCalledTimes(2);
  });

  it('lists validated connected browsers', async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse({
        protocolVersion: 1,
        coordinationVersion: 1,
        browserLeaseSeconds: 30,
        browsers: [
          {
            instanceId: '0123456789abcdef0123456789abcdef',
            browser: 'edge',
            version: '140.0.0.0',
            capabilities: ['capture', 'restore'],
          },
        ],
      }),
    );
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.listBrowsers()).resolves.toEqual([
      {
        instanceId: '0123456789abcdef0123456789abcdef',
        browser: 'edge',
        version: '140.0.0.0',
        capabilities: ['capture', 'restore'],
      },
    ]);
  });

  it('creates a random 128-bit browser instance id', () => {
    expect(createCompanionBrowserInstanceId()).toMatch(/^[0-9a-f]{32}$/u);
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
