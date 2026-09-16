from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, got {count}")
    return text.replace(old, new, 1)


browser_path = Path("apps/chrome-extension/src/browser.ts")
browser = browser_path.read_text()
if "currentBrowserRuntime" not in browser:
    browser = replace_once(
        browser,
        "import { firefoxAdapter } from './firefox-adapter.js';\nimport { isFirefoxUserAgent } from './firefox-runtime.js';\nimport type { BrowserAdapter, RestoreReport } from './webextension-adapter.js';\n\nexport function currentBrowser(userAgent = navigator.userAgent): Browser {\n  if (isFirefoxUserAgent(userAgent)) return 'firefox';\n  return detectChromiumRuntime(userAgent).browser;\n}\n",
        "import { firefoxAdapter } from './firefox-adapter.js';\nimport { detectFirefoxRuntime, isFirefoxUserAgent } from './firefox-runtime.js';\nimport type { BrowserAdapter, BrowserRuntime, RestoreReport } from './webextension-adapter.js';\n\nexport function currentBrowserRuntime(userAgent = navigator.userAgent): BrowserRuntime {\n  if (isFirefoxUserAgent(userAgent)) return detectFirefoxRuntime(userAgent);\n  return detectChromiumRuntime(userAgent);\n}\n\nexport function currentBrowser(userAgent = navigator.userAgent): Browser {\n  return currentBrowserRuntime(userAgent).browser;\n}\n",
        "browser runtime",
    )
    browser_path.write_text(browser)

companion_path = Path("apps/chrome-extension/src/companion.ts")
companion = companion_path.read_text()
if "registerBrowser(" not in companion:
    companion = "import type { Browser } from '@tabsnap/schema';\n\n" + companion
    companion = replace_once(
        companion,
        "export const COMPANION_PROTOCOL_VERSION = 1;\nexport const COMPANION_ORIGIN_PERMISSION = 'http://127.0.0.1/*';\nexport const MAX_COMPANION_SNAPSHOT_BYTES = 65 * 1024 * 1024;\n\nconst PAIRING_PREFIX = `tabsnap-companion:v${COMPANION_PROTOCOL_VERSION}:`;\nconst SESSION_TOKEN_PATTERN = /^[0-9a-f]{64}$/u;\nconst DEFAULT_TIMEOUT_MS = 5_000;\n",
        "export const COMPANION_PROTOCOL_VERSION = 1;\nexport const COMPANION_COORDINATION_VERSION = 1;\nexport const COMPANION_ORIGIN_PERMISSION = 'http://127.0.0.1/*';\nexport const MAX_COMPANION_SNAPSHOT_BYTES = 65 * 1024 * 1024;\nexport const MAX_COMPANION_BROWSER_INSTANCES = 32;\n\nconst PAIRING_PREFIX = `tabsnap-companion:v${COMPANION_PROTOCOL_VERSION}:`;\nconst BROWSER_WIRE_PREFIX = `tabsnap-browser:v${COMPANION_COORDINATION_VERSION}`;\nconst SESSION_TOKEN_PATTERN = /^[0-9a-f]{64}$/u;\nconst BROWSER_INSTANCE_ID_PATTERN = /^[0-9a-f]{32}$/u;\nconst BROWSER_VERSION_PATTERN = /^[0-9A-Za-z._+-]{1,64}$/u;\nconst DEFAULT_TIMEOUT_MS = 5_000;\n",
        "companion constants",
    )
    companion = replace_once(
        companion,
        "export interface CompanionStatus {\n  protocolVersion: number;\n  transport: string;\n  authentication: string;\n}\n\nexport interface CompanionSnapshotEntry {\n",
        "export interface CompanionStatus {\n  protocolVersion: number;\n  transport: string;\n  authentication: string;\n  coordinationVersion?: number;\n  browserLeaseSeconds?: number;\n}\n\nexport type CompanionBrowserCapability = 'capture' | 'restore';\n\nexport interface CompanionBrowserRegistration {\n  instanceId: string;\n  browser: Browser;\n  version?: string;\n  capabilities: CompanionBrowserCapability[];\n}\n\nexport interface CompanionBrowserEntry extends CompanionBrowserRegistration {}\n\nexport interface CompanionSnapshotEntry {\n",
        "companion interfaces",
    )
    old_status = '''    if (
      protocolVersion !== COMPANION_PROTOCOL_VERSION ||
      typeof transport !== 'string' ||
      typeof authentication !== 'string'
    ) {
      throw new Error('Companion protocol status is incompatible.');
    }

    return { protocolVersion, transport, authentication };
  }
'''
    new_status = '''    if (
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
      body: `${BROWSER_WIRE_PREFIX}\\n${instanceId}`,
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
'''
    companion = replace_once(companion, old_status, new_status, "status and coordination methods")
    helper_anchor = "function copyArrayBuffer(bytes: Uint8Array): ArrayBuffer {\n"
    helpers = '''export function createCompanionBrowserInstanceId(
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
  if (
    registration.version !== undefined &&
    !BROWSER_VERSION_PATTERN.test(registration.version)
  ) {
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
  ].join('\\n');
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

'''
    companion = replace_once(companion, helper_anchor, helpers + helper_anchor, "companion helpers")
    companion_path.write_text(companion)

test_path = Path("apps/chrome-extension/src/companion.test.ts")
test = test_path.read_text()
if "registers browser presence" not in test:
    test = replace_once(
        test,
        "  CompanionClient,\n  MAX_COMPANION_SNAPSHOT_BYTES,\n  parseCompanionPairingCode,\n",
        "  CompanionClient,\n  MAX_COMPANION_SNAPSHOT_BYTES,\n  createCompanionBrowserInstanceId,\n  parseCompanionPairingCode,\n",
        "test imports",
    )
    anchor = "  it('lists validated library entries', async () => {\n"
    tests = '''  it('accepts and validates coordination status', async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse({
        protocolVersion: 1,
        transport: 'loopback-http',
        authentication: 'session-bearer',
        coordinationVersion: 1,
        browserLeaseSeconds: 30,
      }),
    );
    const client = new CompanionClient(parseCompanionPairingCode(PAIRING), { fetchImpl });

    await expect(client.status()).resolves.toMatchObject({
      coordinationVersion: 1,
      browserLeaseSeconds: 30,
    });
  });

  it('registers browser presence and sends heartbeats', async () => {
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      expect(init?.method).toBe('POST');
      expect(new Headers(init?.headers).get('Content-Type')).toContain('text/plain');
      if (url.endsWith('/v1/browser/register')) {
        expect(init?.body).toBe(
          'tabsnap-browser:v1\\n0123456789abcdef0123456789abcdef\\nfirefox\\n143.0\\ncapture,restore',
        );
      } else {
        expect(url).toMatch(/\/v1\/browser\/heartbeat$/u);
        expect(init?.body).toBe(
          'tabsnap-browser:v1\\n0123456789abcdef0123456789abcdef',
        );
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

'''
    test = replace_once(test, anchor, tests + anchor, "coordination tests")
    test_path.write_text(test)

current_test_path = Path("apps/chrome-extension/src/current-browser.test.ts")
current_test = current_test_path.read_text()
if "currentBrowserRuntime" not in current_test:
    current_test = replace_once(
        current_test,
        "import { currentBrowser } from './browser.js';\n",
        "import { currentBrowser, currentBrowserRuntime } from './browser.js';\n",
        "runtime test import",
    )
    anchor = "  it('detects Firefox', () => {\n"
    extra = '''  it('exposes runtime version for coordination metadata', () => {
    expect(
      currentBrowserRuntime(
        'Mozilla/5.0 AppleWebKit/537.36 Chrome/140.0.0.0 Safari/537.36 Edg/140.0.3485.54',
      ),
    ).toMatchObject({ browser: 'edge', version: '140.0.3485.54' });
  });

'''
    current_test = replace_once(current_test, anchor, extra + anchor, "runtime metadata test")
    current_test_path.write_text(current_test)

main_path = Path("apps/chrome-extension/src/main.ts")
main = main_path.read_text()
if "startCompanionPresence" not in main:
    main = replace_once(
        main,
        "import { captureWorkspace, currentBrowser, restoreWorkspace } from './browser.js';\n",
        "import {\n  captureWorkspace,\n  currentBrowser,\n  currentBrowserRuntime,\n  restoreWorkspace,\n} from './browser.js';\n",
        "main browser import",
    )
    main = replace_once(
        main,
        "  CompanionClient,\n  COMPANION_ORIGIN_PERMISSION,\n  parseCompanionPairingCode,\n",
        "  CompanionClient,\n  COMPANION_COORDINATION_VERSION,\n  COMPANION_ORIGIN_PERMISSION,\n  createCompanionBrowserInstanceId,\n  parseCompanionPairingCode,\n",
        "main companion import",
    )
    main = replace_once(
        main,
        "let currentSnapshot: TabSnapSnapshot | undefined;\nlet companionClient: CompanionClient | undefined;\nlet busy = false;\n",
        "let currentSnapshot: TabSnapSnapshot | undefined;\nlet companionClient: CompanionClient | undefined;\nlet companionHeartbeatTimer: number | undefined;\nlet busy = false;\n",
        "main state",
    )
    anchor = "async function refreshCompanionLibrary(client: CompanionClient): Promise<void> {\n"
    presence = '''function stopCompanionPresence(): void {
  if (companionHeartbeatTimer !== undefined) {
    window.clearInterval(companionHeartbeatTimer);
    companionHeartbeatTimer = undefined;
  }
}

async function startCompanionPresence(
  client: CompanionClient,
  companionStatus: Awaited<ReturnType<CompanionClient['status']>>,
): Promise<boolean> {
  stopCompanionPresence();
  if (
    companionStatus.coordinationVersion !== COMPANION_COORDINATION_VERSION ||
    companionStatus.browserLeaseSeconds === undefined
  ) {
    return false;
  }

  const runtime = currentBrowserRuntime();
  const registration = {
    instanceId: createCompanionBrowserInstanceId(),
    browser: runtime.browser,
    ...(runtime.version === undefined ? {} : { version: runtime.version }),
    capabilities: ['capture', 'restore'] as const,
  };
  await client.registerBrowser({
    ...registration,
    capabilities: [...registration.capabilities],
  });

  const heartbeatMs = Math.max(
    5_000,
    Math.floor((companionStatus.browserLeaseSeconds * 1000) / 3),
  );
  companionHeartbeatTimer = window.setInterval(() => {
    void client.heartbeatBrowser(registration.instanceId).catch(async () => {
      try {
        await client.registerBrowser({
          ...registration,
          capabilities: [...registration.capabilities],
        });
      } catch {
        // A later heartbeat retries. Snapshot-library operations stay independent.
      }
    });
  }, heartbeatMs);
  return true;
}

'''
    main = replace_once(main, anchor, presence + anchor, "presence helpers")
    old_connect = '''    companionClient = client;
    companionPairing.value = '';
    await refreshCompanionLibrary(client);
    setStatus(
      browser === 'firefox'
        ? 'Portable companion connected for this page session. Firefox data transmission consent and loopback access are enabled for companion mode. The pairing token is kept in memory only.'
        : 'Portable companion connected for this page session. The pairing token is kept in memory only.',
      'success',
    );
'''
    new_connect = '''    const coordinationActive = await startCompanionPresence(client, companionStatus);
    companionClient = client;
    companionPairing.value = '';
    await refreshCompanionLibrary(client);
    const coordinationNote = coordinationActive
      ? ' This browser is registered ephemerally for whole-machine coordination.'
      : ' This companion does not advertise whole-machine coordination; snapshot-library mode still works.';
    setStatus(
      browser === 'firefox'
        ? `Portable companion connected for this page session. Firefox data transmission consent and loopback access are enabled for companion mode. The pairing token is kept in memory only.${coordinationNote}`
        : `Portable companion connected for this page session. The pairing token is kept in memory only.${coordinationNote}`,
      'success',
    );
'''
    main = replace_once(main, old_connect, new_connect, "connect presence")
    main = replace_once(
        main,
        "companionDisconnectButton.addEventListener('click', () => {\n  companionClient = undefined;\n",
        "companionDisconnectButton.addEventListener('click', () => {\n  stopCompanionPresence();\n  companionClient = undefined;\n",
        "disconnect presence",
    )
    main_path.write_text(main)
