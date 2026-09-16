from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'{path}: expected one anchor, found {count}: {old[:100]!r}')
    file.write_text(text.replace(old, new, 1))


path = 'apps/chrome-extension/src/companion.ts'
replace_once(
    path,
    '''export const COMPANION_PROTOCOL_VERSION = 1;
export const COMPANION_COORDINATION_VERSION = 1;
''',
    '''export const COMPANION_PROTOCOL_VERSION = 1;
export const COMPANION_COORDINATION_VERSION = 1;
export const COMPANION_CAPTURE_VERSION = 1;
''',
)
replace_once(
    path,
    '''const BROWSER_WIRE_PREFIX = `tabsnap-browser:v${COMPANION_COORDINATION_VERSION}`;
const SESSION_TOKEN_PATTERN = /^[0-9a-f]{64}$/u;
const BROWSER_INSTANCE_ID_PATTERN = /^[0-9a-f]{32}$/u;
''',
    '''const BROWSER_WIRE_PREFIX = `tabsnap-browser:v${COMPANION_COORDINATION_VERSION}`;
const CAPTURE_WIRE_PREFIX = `tabsnap-capture:v${COMPANION_CAPTURE_VERSION}`;
const SESSION_TOKEN_PATTERN = /^[0-9a-f]{64}$/u;
const BROWSER_INSTANCE_ID_PATTERN = /^[0-9a-f]{32}$/u;
const CAPTURE_JOB_ID_PATTERN = /^[0-9a-f]{32}$/u;
''',
)
replace_once(
    path,
    '''  coordinationVersion?: number;
  browserLeaseSeconds?: number;
}
''',
    '''  coordinationVersion?: number;
  browserLeaseSeconds?: number;
  captureVersion?: number;
}
''',
)
replace_once(
    path,
    '''export type CompanionBrowserEntry = CompanionBrowserRegistration;

export interface CompanionSnapshotEntry {
''',
    '''export type CompanionBrowserEntry = CompanionBrowserRegistration;

export interface CompanionCaptureAssignment {
  jobId: string;
}

export type CompanionCaptureFailure =
  | 'password-required'
  | 'capture-failed'
  | 'encryption-failed';

export interface CompanionSnapshotEntry {
''',
)
replace_once(
    path,
    '''    const coordinationVersion = payload.coordinationVersion;
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
''',
    '''    const coordinationVersion = payload.coordinationVersion;
    const browserLeaseSeconds = payload.browserLeaseSeconds;
    const captureVersion = payload.captureVersion;
    if (coordinationVersion === undefined && browserLeaseSeconds === undefined) {
      if (captureVersion !== undefined) {
        throw new Error('Companion capture status is incompatible.');
      }
      return { protocolVersion, transport, authentication };
    }
    if (
      coordinationVersion !== COMPANION_COORDINATION_VERSION ||
      typeof browserLeaseSeconds !== 'number' ||
      !Number.isSafeInteger(browserLeaseSeconds) ||
      browserLeaseSeconds < 5 ||
      browserLeaseSeconds > 300 ||
      (captureVersion !== undefined && captureVersion !== COMPANION_CAPTURE_VERSION)
    ) {
      throw new Error('Companion coordination status is incompatible.');
    }

    return {
      protocolVersion,
      transport,
      authentication,
      coordinationVersion,
      browserLeaseSeconds,
      ...(captureVersion === undefined ? {} : { captureVersion }),
    };
''',
)
replace_once(
    path,
    '''  async listBrowsers(): Promise<CompanionBrowserEntry[]> {
''',
    '''  async pollCapture(instanceId: string): Promise<CompanionCaptureAssignment | undefined> {
    assertBrowserInstanceId(instanceId);
    const response = await this.#request('/v1/browser/capture/poll', {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain;charset=UTF-8' },
      body: `${CAPTURE_WIRE_PREFIX}\\n${instanceId}`,
    });
    if (response.status === 204) return undefined;

    const payload = await parseJsonObject(
      response,
      'Companion returned an invalid capture assignment.',
    );
    if (
      payload.protocolVersion !== COMPANION_PROTOCOL_VERSION ||
      payload.coordinationVersion !== COMPANION_COORDINATION_VERSION ||
      payload.captureVersion !== COMPANION_CAPTURE_VERSION ||
      typeof payload.jobId !== 'string' ||
      !CAPTURE_JOB_ID_PATTERN.test(payload.jobId)
    ) {
      throw new Error('Companion returned an invalid capture assignment.');
    }
    return { jobId: payload.jobId };
  }

  async submitCaptureResult(
    instanceId: string,
    jobId: string,
    encryptedBytes: Uint8Array,
  ): Promise<void> {
    assertBrowserInstanceId(instanceId);
    assertCaptureJobId(jobId);
    if (encryptedBytes.byteLength === 0) throw new Error('Encrypted capture result is empty.');
    if (encryptedBytes.byteLength > MAX_COMPANION_SNAPSHOT_BYTES) {
      throw new Error('Encrypted capture result exceeds the companion size limit.');
    }

    await this.#request('/v1/browser/capture/result', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/octet-stream',
        'X-TabSnap-Instance': instanceId,
        'X-TabSnap-Job': jobId,
      },
      body: copyArrayBuffer(encryptedBytes),
    });
  }

  async submitCaptureFailure(
    instanceId: string,
    jobId: string,
    reason: CompanionCaptureFailure,
  ): Promise<void> {
    assertBrowserInstanceId(instanceId);
    assertCaptureJobId(jobId);
    if (!isCaptureFailure(reason)) throw new Error('Invalid companion capture failure reason.');

    await this.#request('/v1/browser/capture/failure', {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain;charset=UTF-8' },
      body: `${CAPTURE_WIRE_PREFIX}\\n${instanceId}\\n${jobId}\\n${reason}`,
    });
  }

  async listBrowsers(): Promise<CompanionBrowserEntry[]> {
''',
)
replace_once(
    path,
    '''function encodeBrowserRegistration(registration: CompanionBrowserRegistration): string {
''',
    '''function assertBrowserInstanceId(instanceId: string): void {
  if (!BROWSER_INSTANCE_ID_PATTERN.test(instanceId)) {
    throw new Error('Invalid companion browser instance id.');
  }
}

function assertCaptureJobId(jobId: string): void {
  if (!CAPTURE_JOB_ID_PATTERN.test(jobId)) {
    throw new Error('Invalid companion capture job id.');
  }
}

function encodeBrowserRegistration(registration: CompanionBrowserRegistration): string {
''',
)
replace_once(
    path,
    '''function isBrowserCapability(value: unknown): value is CompanionBrowserCapability {
''',
    '''function isCaptureFailure(value: unknown): value is CompanionCaptureFailure {
  return (
    value === 'password-required' || value === 'capture-failed' || value === 'encryption-failed'
  );
}

function isBrowserCapability(value: unknown): value is CompanionBrowserCapability {
''',
)

path = 'apps/chrome-extension/src/main.ts'
replace_once(
    path,
    '''  decryptSnapshot,
  encryptSnapshot,
  exportSnapshotString,
  importSnapshotString,
} from '@tabsnap/crypto';
''',
    '''  decryptSnapshot,
  encryptSnapshot,
  exportSnapshotString,
  importSnapshotString,
  MIN_PASSWORD_LENGTH,
} from '@tabsnap/crypto';
''',
)
replace_once(
    path,
    '''  CompanionClient,
  COMPANION_COORDINATION_VERSION,
  COMPANION_ORIGIN_PERMISSION,
''',
    '''  CompanionClient,
  COMPANION_CAPTURE_VERSION,
  COMPANION_COORDINATION_VERSION,
  COMPANION_ORIGIN_PERMISSION,
''',
)
replace_once(
    path,
    '''  createCompanionBrowserInstanceId,
  parseCompanionPairingCode,
} from './companion.js';
''',
    '''  createCompanionBrowserInstanceId,
  parseCompanionPairingCode,
  type CompanionCaptureFailure,
} from './companion.js';
''',
)
replace_once(
    path,
    '''let companionClient: CompanionClient | undefined;
let companionHeartbeatTimer: number | undefined;
let busy = false;
''',
    '''let companionClient: CompanionClient | undefined;
let companionHeartbeatTimer: number | undefined;
let companionCaptureTimer: number | undefined;
let companionBrowserInstanceId: string | undefined;
let companionCaptureInFlight = false;
let busy = false;

const COMPANION_CAPTURE_POLL_MS = 1_500;

type PendingCaptureOutcome =
  | {
      kind: 'result';
      instanceId: string;
      jobId: string;
      encrypted: Uint8Array;
    }
  | {
      kind: 'failure';
      instanceId: string;
      jobId: string;
      reason: CompanionCaptureFailure;
    };

let companionPendingCaptureOutcome: PendingCaptureOutcome | undefined;
''',
)
replace_once(
    path,
    '''function stopCompanionPresence(): void {
  if (companionHeartbeatTimer !== undefined) {
    window.clearInterval(companionHeartbeatTimer);
    companionHeartbeatTimer = undefined;
  }
}

async function startCompanionPresence(
''',
    '''function stopCompanionPresence(): void {
  if (companionHeartbeatTimer !== undefined) {
    window.clearInterval(companionHeartbeatTimer);
    companionHeartbeatTimer = undefined;
  }
  if (companionCaptureTimer !== undefined) {
    window.clearInterval(companionCaptureTimer);
    companionCaptureTimer = undefined;
  }
  companionBrowserInstanceId = undefined;
  companionPendingCaptureOutcome = undefined;
}

async function submitPendingCaptureOutcome(
  client: CompanionClient,
  outcome: PendingCaptureOutcome,
): Promise<void> {
  if (outcome.kind === 'result') {
    await client.submitCaptureResult(outcome.instanceId, outcome.jobId, outcome.encrypted);
  } else {
    await client.submitCaptureFailure(outcome.instanceId, outcome.jobId, outcome.reason);
  }
}

async function pollCompanionCapture(client: CompanionClient, instanceId: string): Promise<void> {
  if (companionCaptureInFlight || busy || companionBrowserInstanceId !== instanceId) return;
  companionCaptureInFlight = true;
  let claimed = false;

  try {
    const pending = companionPendingCaptureOutcome;
    if (pending !== undefined && pending.instanceId === instanceId) {
      await submitPendingCaptureOutcome(client, pending);
      if (
        companionBrowserInstanceId === instanceId &&
        companionPendingCaptureOutcome === pending
      ) {
        companionPendingCaptureOutcome = undefined;
        setStatus(
          pending.kind === 'result'
            ? `Whole-machine capture ${pending.jobId} submitted as encrypted bytes.`
            : `Whole-machine capture ${pending.jobId} failure reported to the companion.`,
          pending.kind === 'result' ? 'success' : 'info',
        );
      }
      return;
    }

    const assignment = await client.pollCapture(instanceId);
    if (assignment === undefined || companionBrowserInstanceId !== instanceId) return;

    claimed = true;
    busy = true;
    syncButtons();
    setStatus(`Whole-machine capture ${assignment.jobId}: capturing this browser…`);

    const capturePassword = password();
    let outcome: PendingCaptureOutcome;
    if (capturePassword.length < MIN_PASSWORD_LENGTH) {
      outcome = {
        kind: 'failure',
        instanceId,
        jobId: assignment.jobId,
        reason: 'password-required',
      };
    } else {
      let snapshot: TabSnapSnapshot;
      try {
        snapshot = await captureWorkspace();
      } catch {
        outcome = {
          kind: 'failure',
          instanceId,
          jobId: assignment.jobId,
          reason: 'capture-failed',
        };
        if (companionBrowserInstanceId !== instanceId) return;
        companionPendingCaptureOutcome = outcome;
        try {
          await submitPendingCaptureOutcome(client, outcome);
          if (companionPendingCaptureOutcome === outcome) {
            companionPendingCaptureOutcome = undefined;
          }
        } catch {
          // Retry the bounded failure report from memory on the next poll tick.
        }
        setStatus(
          `Whole-machine capture ${assignment.jobId} failed while reading this browser.`,
          'error',
        );
        return;
      }

      try {
        outcome = {
          kind: 'result',
          instanceId,
          jobId: assignment.jobId,
          encrypted: await encryptSnapshot(snapshot, capturePassword),
        };
      } catch {
        outcome = {
          kind: 'failure',
          instanceId,
          jobId: assignment.jobId,
          reason: 'encryption-failed',
        };
      }
    }

    if (companionBrowserInstanceId !== instanceId) return;
    companionPendingCaptureOutcome = outcome;
    try {
      await submitPendingCaptureOutcome(client, outcome);
      if (companionPendingCaptureOutcome === outcome) {
        companionPendingCaptureOutcome = undefined;
      }
      if (outcome.kind === 'result') {
        setStatus(
          `Whole-machine capture ${assignment.jobId} completed for this browser. Only encrypted bytes were sent to the companion.`,
          'success',
        );
      } else if (outcome.reason === 'password-required') {
        setStatus(
          'Whole-machine capture needs an encryption password in this browser page. Start a new capture job after entering it.',
          'info',
        );
      } else {
        setStatus(
          `Whole-machine capture ${assignment.jobId} could not encrypt this browser workspace.`,
          'error',
        );
      }
    } catch {
      setStatus(
        outcome.kind === 'result'
          ? 'Encrypted whole-machine capture result is held in memory and will retry submission.'
          : 'Whole-machine capture failure report is held in memory and will retry submission.',
        'info',
      );
    }
  } catch {
    // Polling failures are transient. Heartbeats and snapshot-library mode stay independent.
  } finally {
    if (claimed) {
      busy = false;
      syncButtons();
    }
    companionCaptureInFlight = false;
  }
}

async function startCompanionPresence(
''',
)
replace_once(
    path,
    '''  await client.registerBrowser({
    ...registration,
    capabilities: [...registration.capabilities],
  });

  const heartbeatMs = Math.max(5_000, Math.floor((companionStatus.browserLeaseSeconds * 1000) / 3));
''',
    '''  await client.registerBrowser({
    ...registration,
    capabilities: [...registration.capabilities],
  });
  companionBrowserInstanceId = registration.instanceId;

  const heartbeatMs = Math.max(5_000, Math.floor((companionStatus.browserLeaseSeconds * 1000) / 3));
''',
)
replace_once(
    path,
    '''  }, heartbeatMs);
  return true;
}
''',
    '''  }, heartbeatMs);

  if (companionStatus.captureVersion === COMPANION_CAPTURE_VERSION) {
    companionCaptureTimer = window.setInterval(() => {
      void pollCompanionCapture(client, registration.instanceId);
    }, COMPANION_CAPTURE_POLL_MS);
    void pollCompanionCapture(client, registration.instanceId);
  }
  return true;
}
''',
)
replace_once(
    path,
    '''    const coordinationNote = coordinationActive
      ? ' This browser is registered ephemerally for whole-machine coordination.'
      : ' This companion does not advertise whole-machine coordination; snapshot-library mode still works.';
''',
    '''    const captureActive = companionStatus.captureVersion === COMPANION_CAPTURE_VERSION;
    const coordinationNote = coordinationActive
      ? ` This browser is registered ephemerally for whole-machine coordination.${
          captureActive
            ? ' Coordinated capture polling is active while this page stays open.'
            : ' This companion does not advertise coordinated capture yet.'
        }`
      : ' This companion does not advertise whole-machine coordination; snapshot-library mode still works.';
''',
)

path = 'apps/chrome-extension/src/companion.test.ts'
replace_once(
    path,
    '''        coordinationVersion: 1,
        browserLeaseSeconds: 30,
      }),
''',
    '''        coordinationVersion: 1,
        browserLeaseSeconds: 30,
        captureVersion: 1,
      }),
''',
)
replace_once(
    path,
    '''      coordinationVersion: 1,
      browserLeaseSeconds: 30,
    });
  });

  it('registers browser presence and sends heartbeats', async () => {
''',
    '''      coordinationVersion: 1,
      browserLeaseSeconds: 30,
      captureVersion: 1,
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

    await expect(
      client.pollCapture('0123456789abcdef0123456789abcdef'),
    ).resolves.toEqual({ jobId: '11111111111111111111111111111111' });
    await expect(
      client.pollCapture('0123456789abcdef0123456789abcdef'),
    ).resolves.toBeUndefined();

    for (const call of fetchImpl.mock.calls) {
      expect(String(call[0])).toMatch(/\/v1\/browser\/capture\/poll$/u);
      expect(call[1]?.method).toBe('POST');
      expect(call[1]?.body).toBe(
        'tabsnap-capture:v1\\n0123456789abcdef0123456789abcdef',
      );
    }
  });

  it('submits opaque coordinated capture bytes and bounded failure codes', async () => {
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/v1/browser/capture/result')) {
        const headers = new Headers(init?.headers);
        expect(headers.get('Content-Type')).toBe('application/octet-stream');
        expect(headers.get('X-TabSnap-Instance')).toBe(
          '0123456789abcdef0123456789abcdef',
        );
        expect(headers.get('X-TabSnap-Job')).toBe('11111111111111111111111111111111');
        expect(Array.from(new Uint8Array(init?.body as ArrayBuffer))).toEqual([1, 2, 3, 255]);
      } else {
        expect(url).toMatch(/\/v1\/browser\/capture\/failure$/u);
        expect(init?.body).toBe(
          'tabsnap-capture:v1\\n0123456789abcdef0123456789abcdef\\n11111111111111111111111111111111\\npassword-required',
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

  it('registers browser presence and sends heartbeats', async () => {
''',
)
