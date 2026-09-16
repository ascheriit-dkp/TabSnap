import type { Browser } from '@tabsnap/schema';

export const COMPANION_PROTOCOL_VERSION = 1;
export const COMPANION_COORDINATION_VERSION = 1;
export const COMPANION_ORIGIN_PERMISSION = 'http://127.0.0.1/*';
export const MAX_COMPANION_SNAPSHOT_BYTES = 65 * 1024 * 1024;
export const MAX_COMPANION_BROWSER_INSTANCES = 32;

const PAIRING_PREFIX = `tabsnap-companion:v${COMPANION_PROTOCOL_VERSION}:`;
const BROWSER_WIRE_PREFIX = `tabsnap-browser:v${COMPANION_COORDINATION_VERSION}`;
const SESSION_TOKEN_PATTERN = /^[0-9a-f]{64}$/u;
const BROWSER_INSTANCE_ID_PATTERN = /^[0-9a-f]{32}$/u;
const BROWSER_VERSION_PATTERN = /^[0-9A-Za-z._+-]{1,64}$/u;
const DEFAULT_TIMEOUT_MS = 5_000;

export interface CompanionPairing {
  endpoint: string;
  token: string;
}

export interface CompanionStatus {
  protocolVersion: number;
  transport: string;
  authentication: string;
  coordinationVersion?: number;
  browserLeaseSeconds?: number;
}

export type CompanionBrowserCapability = 'capture' | 'restore';

export interface CompanionBrowserRegistration {
  instanceId: string;
  browser: Browser;
  version?: string;
  capabilities: CompanionBrowserCapability[];
}

export interface CompanionBrowserEntry extends CompanionBrowserRegistration {}

export interface CompanionSnapshotEntry {
  name: string;
  size: number;
}

type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

export function parseCompanionPairingCode(value: string): CompanionPairing {
  const pairingCode = value.trim();
  if (!pairingCode.startsWith(PAIRING_PREFIX)) {
    throw new Error('Invalid companion pairing code or unsupported protocol version.');
  }

  const remainder = pairingCode.slice(PAIRING_PREFIX.length);
  const separator = remainder.indexOf(':');
  if (separator <= 0 || separator !== remainder.lastIndexOf(':')) {
    throw new Error('Invalid companion pairing code.');
  }

  const portText = remainder.slice(0, separator);
  const token = remainder.slice(separator + 1);
  if (!/^\d{1,5}$/u.test(portText)) throw new Error('Invalid companion port.');

  const port = Number(portText);
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new Error('Invalid companion port.');
  }
  if (!SESSION_TOKEN_PATTERN.test(token)) {
    throw new Error('Invalid companion session token.');
  }

  return {
    endpoint: `http://127.0.0.1:${port}`,
    token,
  };
}

export class CompanionClient {
  readonly endpoint: string;
  readonly #token: string;
  readonly #fetch: FetchLike;
  readonly #timeoutMs: number;

  constructor(
    pairing: CompanionPairing,
    options: { fetchImpl?: FetchLike; timeoutMs?: number } = {},
  ) {
    if (!/^http:\/\/127\.0\.0\.1:\d{1,5}$/u.test(pairing.endpoint)) {
      throw new Error('Companion endpoint must use IPv4 loopback.');
    }
    if (!SESSION_TOKEN_PATTERN.test(pairing.token)) {
      throw new Error('Invalid companion session token.');
    }

    this.endpoint = pairing.endpoint;
    this.#token = pairing.token;
    this.#fetch = options.fetchImpl ?? globalThis.fetch.bind(globalThis);
    this.#timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  }

  async status(): Promise<CompanionStatus> {
    const response = await this.#request('/v1/status');
    const payload = await parseJsonObject(
      response,
      'Companion returned an invalid status response.',
    );
    const protocolVersion = payload.protocolVersion;
    const transport = payload.transport;
    const authentication = payload.authentication;

    if (
      protocolVersion !== COMPANION_PROTOCOL_VERSION ||
      typeof transport !== 'string' ||
      typeof authentication !== 'string'
    ) {
      throw new Error('Companion protocol status is incompatible.');
    }

    const coordinationVersion = payload.coordinationVersion;
    const browserLeaseSeconds = payload.browserLeaseSeconds;
    if (coordinationVersion === undefined && browserLeaseSeconds === undefined) {
      return { protocolVersion, transport, authentication };
    }
    if (
      coordinationVersion !== COMPANION_COORDINATION_VERSION ||
      typeof browserLeaseSeconds !== 'number' ||
      !Number.isSafeInteger(browserLeaseSeconds) ||
      browserLeaseSeconds < 5 ||
      browserLeaseSeconds > 300
    ) {
      throw new Error('Companion coordination status is incompatible.');
    }

    return {
      protocolVersion,
      transport,
      authentication,
      coordinationVersion,
      browserLeaseSeconds,
    };
  }

  async registerBrowser(registration: CompanionBrowserRegistration): Promise<void> {
    const body = encodeBrowserRegistration(registration);
    await this.#request('/v1/browser/register', {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain;charset=UTF-8' },
      body,
    });
  }

  async heartbeatBrowser(instanceId: string): Promise<void> {
    if (!BROWSER_INSTANCE_ID_PATTERN.test(instanceId)) {
      throw new Error('Invalid companion browser instance id.');
    }
    await this.#request('/v1/browser/heartbeat', {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain;charset=UTF-8' },
      body: `${BROWSER_WIRE_PREFIX}\n${instanceId}`,
    });
  }

  async listBrowsers(): Promise<CompanionBrowserEntry[]> {
    const response = await this.#request('/v1/browsers');
    const payload = await parseJsonObject(response, 'Companion returned an invalid browser list.');
    if (
      payload.protocolVersion !== COMPANION_PROTOCOL_VERSION ||
      payload.coordinationVersion !== COMPANION_COORDINATION_VERSION ||
      !Array.isArray(payload.browsers) ||
      payload.browsers.length > MAX_COMPANION_BROWSER_INSTANCES
    ) {
      throw new Error('Companion browser list is incompatible.');
    }

    return payload.browsers.map(parseBrowserEntry);
  }

  async listSnapshots(): Promise<CompanionSnapshotEntry[]> {
    const response = await this.#request('/v1/snapshots');
    const payload = await parseJsonObject(response, 'Companion returned an invalid snapshot list.');
    if (
      payload.protocolVersion !== COMPANION_PROTOCOL_VERSION ||
      !Array.isArray(payload.snapshots)
    ) {
      throw new Error('Companion snapshot list is incompatible.');
    }

    return payload.snapshots.map((candidate) => {
      if (!isRecord(candidate)) throw new Error('Companion returned an invalid snapshot entry.');
      const { name, size } = candidate;
      if (
        typeof name !== 'string' ||
        name.length === 0 ||
        typeof size !== 'number' ||
        !Number.isSafeInteger(size) ||
        size < 0 ||
        size > MAX_COMPANION_SNAPSHOT_BYTES
      ) {
        throw new Error('Companion returned an invalid snapshot entry.');
      }
      return { name, size };
    });
  }

  async storeSnapshot(name: string, encryptedBytes: Uint8Array): Promise<CompanionSnapshotEntry> {
    if (name.trim().length === 0) throw new Error('Snapshot name must not be empty.');
    if (encryptedBytes.byteLength === 0) throw new Error('Encrypted snapshot is empty.');
    if (encryptedBytes.byteLength > MAX_COMPANION_SNAPSHOT_BYTES) {
      throw new Error('Encrypted snapshot exceeds the companion size limit.');
    }

    const response = await this.#request('/v1/snapshot', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/octet-stream',
        'X-TabSnap-Name': name,
      },
      body: copyArrayBuffer(encryptedBytes),
    });
    const payload = await parseJsonObject(
      response,
      'Companion returned an invalid store response.',
    );
    if (
      typeof payload.name !== 'string' ||
      typeof payload.size !== 'number' ||
      !Number.isSafeInteger(payload.size) ||
      payload.size < 0 ||
      payload.size > MAX_COMPANION_SNAPSHOT_BYTES
    ) {
      throw new Error('Companion returned an invalid store response.');
    }
    return { name: payload.name, size: payload.size };
  }

  async loadSnapshot(name: string): Promise<Uint8Array> {
    if (name.trim().length === 0) throw new Error('Choose a companion snapshot first.');

    const response = await this.#request('/v1/snapshot', {
      headers: { 'X-TabSnap-Name': name },
    });
    const contentType = response.headers
      .get('content-type')
      ?.split(';', 1)[0]
      ?.trim()
      .toLowerCase();
    if (contentType !== 'application/octet-stream') {
      throw new Error('Companion returned an invalid snapshot response.');
    }

    const announcedLength = response.headers.get('content-length');
    if (announcedLength !== null) {
      const size = Number(announcedLength);
      if (!Number.isSafeInteger(size) || size < 0 || size > MAX_COMPANION_SNAPSHOT_BYTES) {
        throw new Error('Companion snapshot exceeds the size limit.');
      }
    }

    const buffer = await response.arrayBuffer();
    if (buffer.byteLength > MAX_COMPANION_SNAPSHOT_BYTES) {
      throw new Error('Companion snapshot exceeds the size limit.');
    }
    if (buffer.byteLength === 0) throw new Error('Companion returned an empty snapshot.');
    return new Uint8Array(buffer);
  }

  async #request(path: string, init: RequestInit = {}): Promise<Response> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.#timeoutMs);
    const headers = new Headers(init.headers);
    headers.set('Authorization', `Bearer ${this.#token}`);

    try {
      const response = await this.#fetch(`${this.endpoint}${path}`, {
        ...init,
        headers,
        signal: controller.signal,
        cache: 'no-store',
        credentials: 'omit',
        redirect: 'error',
      });
      if (!response.ok) {
        throw new Error(await responseError(response));
      }
      return response;
    } catch (error) {
      if (controller.signal.aborted) {
        throw new Error('Companion request timed out.', { cause: error });
      }
      throw error;
    } finally {
      clearTimeout(timeout);
    }
  }
}

export function createCompanionBrowserInstanceId(
  cryptoImpl: Pick<Crypto, 'getRandomValues'> = globalThis.crypto,
): string {
  const bytes = new Uint8Array(16);
  cryptoImpl.getRandomValues(bytes);
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

function encodeBrowserRegistration(registration: CompanionBrowserRegistration): string {
  if (!BROWSER_INSTANCE_ID_PATTERN.test(registration.instanceId)) {
    throw new Error('Invalid companion browser instance id.');
  }
  if (!isBrowser(registration.browser)) {
    throw new Error('Invalid companion browser kind.');
  }
  if (registration.version !== undefined && !BROWSER_VERSION_PATTERN.test(registration.version)) {
    throw new Error('Invalid companion browser version.');
  }
  if (!validCapabilities(registration.capabilities)) {
    throw new Error('Invalid companion browser capabilities.');
  }

  return [
    BROWSER_WIRE_PREFIX,
    registration.instanceId,
    registration.browser,
    registration.version ?? '-',
    registration.capabilities.join(','),
  ].join('\n');
}

function parseBrowserEntry(value: unknown): CompanionBrowserEntry {
  if (!isRecord(value)) throw new Error('Companion returned an invalid browser entry.');
  const { instanceId, browser, version, capabilities } = value;
  if (
    typeof instanceId !== 'string' ||
    !BROWSER_INSTANCE_ID_PATTERN.test(instanceId) ||
    !isBrowser(browser) ||
    !Array.isArray(capabilities) ||
    !validCapabilities(capabilities) ||
    !(version === null || (typeof version === 'string' && BROWSER_VERSION_PATTERN.test(version)))
  ) {
    throw new Error('Companion returned an invalid browser entry.');
  }

  return {
    instanceId,
    browser,
    ...(version === null ? {} : { version }),
    capabilities,
  };
}

function isBrowser(value: unknown): value is Browser {
  return value === 'chrome' || value === 'edge' || value === 'firefox';
}

function isBrowserCapability(value: unknown): value is CompanionBrowserCapability {
  return value === 'capture' || value === 'restore';
}

function validCapabilities(value: unknown[]): value is CompanionBrowserCapability[] {
  if (value.length === 0 || value.length > 2 || !value.every(isBrowserCapability)) return false;
  return new Set(value).size === value.length;
}

function copyArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

async function parseJsonObject(
  response: Response,
  fallback: string,
): Promise<Record<string, unknown>> {
  let value: unknown;
  try {
    value = await response.json();
  } catch {
    throw new Error(fallback);
  }
  if (!isRecord(value)) throw new Error(fallback);
  return value;
}

async function responseError(response: Response): Promise<string> {
  try {
    const value: unknown = await response.json();
    if (isRecord(value) && typeof value.error === 'string' && value.error.length > 0) {
      return `Companion error (${response.status}): ${value.error}`;
    }
  } catch {
    // Fall back to the status below.
  }
  return `Companion request failed with HTTP ${response.status}.`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
