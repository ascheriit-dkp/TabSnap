import {
  decryptSnapshot,
  encryptSnapshot,
  exportSnapshotString,
  importSnapshotString,
} from '@tabsnap/crypto';
import type { TabSnapSnapshot } from '@tabsnap/schema';

import { captureWorkspace, currentBrowser, restoreWorkspace } from './browser.js';
import {
  CompanionClient,
  COMPANION_ORIGIN_PERMISSION,
  parseCompanionPairingCode,
} from './companion.js';
import { assessCrossBrowserCompatibility } from './cross-browser.js';
import './style.css';

const MAX_FILE_BYTES = 65 * 1024 * 1024;

function element<T extends HTMLElement>(id: string): T {
  const value = document.getElementById(id);
  if (value === null) throw new Error(`Missing UI element: ${id}.`);
  return value as T;
}

const captureButton = element<HTMLButtonElement>('capture');
const exportStringButton = element<HTMLButtonElement>('export-string');
const exportFileButton = element<HTMLButtonElement>('export-file');
const copyStringButton = element<HTMLButtonElement>('copy-string');
const importStringButton = element<HTMLButtonElement>('import-string');
const restoreButton = element<HTMLButtonElement>('restore');
const passwordInput = element<HTMLInputElement>('password');
const outputString = element<HTMLTextAreaElement>('output-string');
const inputString = element<HTMLTextAreaElement>('input-string');
const inputFile = element<HTMLInputElement>('input-file');
const preview = element<HTMLDivElement>('preview');
const status = element<HTMLPreElement>('status');
const companionState = element<HTMLSpanElement>('companion-state');
const companionPairing = element<HTMLInputElement>('companion-pairing');
const companionConnectButton = element<HTMLButtonElement>('companion-connect');
const companionDisconnectButton = element<HTMLButtonElement>('companion-disconnect');
const companionRefreshButton = element<HTMLButtonElement>('companion-refresh');
const companionName = element<HTMLInputElement>('companion-name');
const companionSendButton = element<HTMLButtonElement>('companion-send');
const companionSnapshots = element<HTMLSelectElement>('companion-snapshots');
const companionLoadButton = element<HTMLButtonElement>('companion-load');

let currentSnapshot: TabSnapSnapshot | undefined;
let companionClient: CompanionClient | undefined;
let busy = false;

function syncButtons(): void {
  const hasSnapshot = currentSnapshot !== undefined;
  const connected = companionClient !== undefined;
  const hasCompanionSelection = connected && companionSnapshots.value.length > 0;

  captureButton.disabled = busy;
  exportStringButton.disabled = busy || !hasSnapshot;
  exportFileButton.disabled = busy || !hasSnapshot;
  importStringButton.disabled = busy;
  inputFile.disabled = busy;
  restoreButton.disabled = busy || !hasSnapshot;
  copyStringButton.disabled = busy || outputString.value.length === 0;

  companionPairing.disabled = busy || connected;
  companionConnectButton.disabled = busy || connected;
  companionDisconnectButton.disabled = busy || !connected;
  companionRefreshButton.disabled = busy || !connected;
  companionName.disabled = busy || !connected;
  companionSendButton.disabled = busy || !connected || !hasSnapshot;
  companionSnapshots.disabled = busy || !connected;
  companionLoadButton.disabled = busy || !hasCompanionSelection;

  companionState.textContent = connected ? 'Connected' : 'Disconnected';
  companionState.dataset.connected = connected ? 'true' : 'false';
}

function setStatus(message: string, kind: 'info' | 'success' | 'error' = 'info'): void {
  status.textContent = message;
  status.dataset.kind = kind;
}

function snapshotStats(snapshot: TabSnapSnapshot): {
  windows: number;
  tabs: number;
  groups: number;
} {
  return {
    windows: snapshot.windows.length,
    tabs: snapshot.windows.reduce((total, window) => total + window.tabs.length, 0),
    groups: snapshot.windows.reduce((total, window) => total + window.groups.length, 0),
  };
}

function showSnapshot(snapshot: TabSnapSnapshot, origin: string): void {
  currentSnapshot = snapshot;
  const stats = snapshotStats(snapshot);
  const compatibility = assessCrossBrowserCompatibility(snapshot, currentBrowser());
  const lines = [
    `${origin}: ${stats.windows} window${stats.windows === 1 ? '' : 's'}, ${stats.tabs} tab${stats.tabs === 1 ? '' : 's'}, ${stats.groups} group${stats.groups === 1 ? '' : 's'}`,
    `Source: ${snapshot.source.browser}${snapshot.source.browserVersion === undefined ? '' : ` ${snapshot.source.browserVersion}`} on ${snapshot.source.platform}`,
    `Captured: ${new Date(snapshot.createdAt).toLocaleString()}`,
  ];

  if (compatibility.crossBrowser) {
    lines.push(
      '',
      `Cross-browser restore: best effort (${compatibility.source} → ${compatibility.target}).`,
      compatibility.knownNonPortableTabs === 0
        ? 'No known non-portable tabs were found.'
        : `${compatibility.knownNonPortableTabs} known non-portable tab${compatibility.knownNonPortableTabs === 1 ? '' : 's'} may be skipped.`,
      ...compatibility.notes,
    );
  }

  preview.classList.remove('empty');
  preview.textContent = lines.join('\n');
  syncButtons();
}

function password(): string {
  return passwordInput.value;
}

function bytesBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

function defaultSnapshotName(snapshot: TabSnapSnapshot): string {
  const stamp = new Date(snapshot.createdAt).toISOString().replaceAll(':', '-');
  return `tabsnap-${stamp}`;
}

function downloadSnapshot(bytes: Uint8Array, snapshot: TabSnapSnapshot): void {
  const blob = new Blob([bytesBuffer(bytes)], { type: 'application/octet-stream' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `${defaultSnapshotName(snapshot)}.tabsnap`;
  anchor.style.display = 'none';
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

function requireCompanion(): CompanionClient {
  if (companionClient === undefined) throw new Error('Connect the portable companion first.');
  return companionClient;
}

async function refreshCompanionLibrary(client: CompanionClient): Promise<void> {
  const previous = companionSnapshots.value;
  const entries = await client.listSnapshots();
  companionSnapshots.replaceChildren();

  if (entries.length === 0) {
    const option = document.createElement('option');
    option.value = '';
    option.textContent = 'Library is empty';
    companionSnapshots.append(option);
  } else {
    for (const entry of entries) {
      const option = document.createElement('option');
      option.value = entry.name;
      option.textContent = `${entry.name} (${formatBytes(entry.size)})`;
      companionSnapshots.append(option);
    }
    const previousOption = Array.from(companionSnapshots.options).find(
      (option) => option.value === previous,
    );
    if (previousOption !== undefined) companionSnapshots.value = previousOption.value;
  }

  syncButtons();
}

function clearCompanionLibrary(): void {
  companionSnapshots.replaceChildren();
  const option = document.createElement('option');
  option.value = '';
  option.textContent = 'No snapshots loaded';
  companionSnapshots.append(option);
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kib = bytes / 1024;
  if (kib < 1024) return `${kib.toFixed(1)} KiB`;
  return `${(kib / 1024).toFixed(1)} MiB`;
}

async function hasCompanionOriginPermission(): Promise<boolean> {
  const requiredOrigins = chrome.runtime.getManifest().host_permissions ?? [];
  if (requiredOrigins.includes(COMPANION_ORIGIN_PERMISSION)) return true;

  return new Promise((resolvePermission) => {
    chrome.permissions.contains({ origins: [COMPANION_ORIGIN_PERMISSION] }, resolvePermission);
  });
}

async function run(action: string, operation: () => Promise<void>): Promise<void> {
  if (busy) return;
  busy = true;
  syncButtons();
  setStatus(`${action}…`);

  try {
    await operation();
  } catch (error) {
    setStatus(error instanceof Error ? error.message : 'Unexpected error.', 'error');
  } finally {
    busy = false;
    syncButtons();
  }
}

captureButton.addEventListener('click', () => {
  void run('Capturing workspace', async () => {
    const snapshot = await captureWorkspace();
    showSnapshot(snapshot, 'Captured');
    setStatus('Workspace captured locally. Nothing has been uploaded.', 'success');
  });
});

exportStringButton.addEventListener('click', () => {
  void run('Encrypting snapshot', async () => {
    if (currentSnapshot === undefined) throw new Error('Capture or import a snapshot first.');
    outputString.value = await exportSnapshotString(currentSnapshot, password());
    syncButtons();

    try {
      await navigator.clipboard.writeText(outputString.value);
      setStatus('Encrypted string created and copied to the clipboard.', 'success');
    } catch {
      setStatus('Encrypted string created. Use the Copy button to copy it.', 'success');
    }
  });
});

copyStringButton.addEventListener('click', () => {
  void run('Copying encrypted string', async () => {
    await navigator.clipboard.writeText(outputString.value);
    setStatus('Encrypted string copied.', 'success');
  });
});

exportFileButton.addEventListener('click', () => {
  void run('Encrypting file', async () => {
    if (currentSnapshot === undefined) throw new Error('Capture or import a snapshot first.');
    const encrypted = await encryptSnapshot(currentSnapshot, password());
    downloadSnapshot(encrypted, currentSnapshot);
    setStatus('Encrypted .tabsnap download started.', 'success');
  });
});

importStringButton.addEventListener('click', () => {
  void run('Decrypting string', async () => {
    const snapshot = await importSnapshotString(inputString.value, password());
    showSnapshot(snapshot, 'Imported');
    setStatus(
      'Encrypted string validated and decrypted locally. Review the preview before restore.',
      'success',
    );
  });
});

inputFile.addEventListener('change', () => {
  const file = inputFile.files?.[0];
  if (file === undefined) return;

  void run('Decrypting file', async () => {
    try {
      if (file.size > MAX_FILE_BYTES) throw new Error('The .tabsnap file is too large.');
      const bytes = new Uint8Array(await file.arrayBuffer());
      const snapshot = await decryptSnapshot(bytes, password());
      showSnapshot(snapshot, 'Imported');
      setStatus(
        'Encrypted file validated and decrypted locally. Review the preview before restore.',
        'success',
      );
    } finally {
      inputFile.value = '';
    }
  });
});

restoreButton.addEventListener('click', () => {
  void run('Restoring workspace', async () => {
    if (currentSnapshot === undefined) throw new Error('Capture or import a snapshot first.');
    const report = await restoreWorkspace(currentSnapshot);
    const lines = [
      `Restored ${report.createdWindows} window${report.createdWindows === 1 ? '' : 's'} and ${report.createdTabs} tab${report.createdTabs === 1 ? '' : 's'}.`,
      report.skippedTabs === 0
        ? 'No tabs were skipped.'
        : `${report.skippedTabs} tab${report.skippedTabs === 1 ? '' : 's'} skipped.`,
    ];

    if (report.warnings.length > 0) {
      lines.push('', 'Warnings:', ...report.warnings.slice(0, 8));
      if (report.warnings.length > 8) lines.push(`…and ${report.warnings.length - 8} more.`);
    }

    setStatus(lines.join('\n'), report.warnings.length === 0 ? 'success' : 'info');
  });
});

companionConnectButton.addEventListener('click', () => {
  void run('Connecting to portable companion', async () => {
    const pairing = parseCompanionPairingCode(companionPairing.value);
    const alreadyGranted = await hasCompanionOriginPermission();
    const granted =
      alreadyGranted ||
      (await chrome.permissions.request({ origins: [COMPANION_ORIGIN_PERMISSION] }));
    if (!granted) throw new Error('Loopback permission was not granted. Companion mode stays off.');

    const client = new CompanionClient(pairing);
    const companionStatus = await client.status();
    if (
      companionStatus.transport !== 'loopback-http' ||
      companionStatus.authentication !== 'session-bearer'
    ) {
      throw new Error('Connected process is not a compatible TabSnap companion.');
    }

    companionClient = client;
    companionPairing.value = '';
    await refreshCompanionLibrary(client);
    setStatus(
      'Portable companion connected for this page session. The pairing token is kept in memory only.',
      'success',
    );
  });
});

companionDisconnectButton.addEventListener('click', () => {
  companionClient = undefined;
  companionPairing.value = '';
  clearCompanionLibrary();
  syncButtons();
  setStatus('Portable companion disconnected. Extension-only mode remains available.', 'success');
});

companionRefreshButton.addEventListener('click', () => {
  void run('Refreshing companion library', async () => {
    await refreshCompanionLibrary(requireCompanion());
    setStatus('Companion library refreshed.', 'success');
  });
});

companionSnapshots.addEventListener('change', syncButtons);

companionSendButton.addEventListener('click', () => {
  void run('Encrypting and saving to companion', async () => {
    if (currentSnapshot === undefined) throw new Error('Capture or import a snapshot first.');
    const client = requireCompanion();
    const encrypted = await encryptSnapshot(currentSnapshot, password());
    const requestedName = companionName.value.trim() || defaultSnapshotName(currentSnapshot);
    const entry = await client.storeSnapshot(requestedName, encrypted);
    companionName.value = '';
    await refreshCompanionLibrary(client);
    companionSnapshots.value = entry.name;
    syncButtons();
    setStatus(
      `Saved ${entry.name} to the local companion as encrypted bytes. The password never left this extension.`,
      'success',
    );
  });
});

companionLoadButton.addEventListener('click', () => {
  void run('Loading encrypted snapshot from companion', async () => {
    const client = requireCompanion();
    const selected = companionSnapshots.value;
    if (selected.length === 0) throw new Error('Choose a companion snapshot first.');
    const encrypted = await client.loadSnapshot(selected);
    const snapshot = await decryptSnapshot(encrypted, password());
    showSnapshot(snapshot, `Companion ${selected}`);
    setStatus(
      'Encrypted snapshot loaded from the local companion and decrypted inside the extension. Review the preview before restore.',
      'success',
    );
  });
});

clearCompanionLibrary();
syncButtons();
