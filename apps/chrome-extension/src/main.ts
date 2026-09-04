import { decryptSnapshot, encryptSnapshot, exportSnapshotString, importSnapshotString } from '@tabsnap/crypto';
import type { TabSnapSnapshot } from '@tabsnap/schema';

import { captureWorkspace, restoreWorkspace } from './browser.js';
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

let currentSnapshot: TabSnapSnapshot | undefined;
let busy = false;

function syncButtons(): void {
  const hasSnapshot = currentSnapshot !== undefined;
  captureButton.disabled = busy;
  exportStringButton.disabled = busy || !hasSnapshot;
  exportFileButton.disabled = busy || !hasSnapshot;
  importStringButton.disabled = busy;
  inputFile.disabled = busy;
  restoreButton.disabled = busy || !hasSnapshot;
  copyStringButton.disabled = busy || outputString.value.length === 0;
}

function setStatus(message: string, kind: 'info' | 'success' | 'error' = 'info'): void {
  status.textContent = message;
  status.dataset.kind = kind;
}

function snapshotStats(snapshot: TabSnapSnapshot): { windows: number; tabs: number; groups: number } {
  return {
    windows: snapshot.windows.length,
    tabs: snapshot.windows.reduce((total, window) => total + window.tabs.length, 0),
    groups: snapshot.windows.reduce((total, window) => total + window.groups.length, 0),
  };
}

function showSnapshot(snapshot: TabSnapSnapshot, origin: string): void {
  currentSnapshot = snapshot;
  const stats = snapshotStats(snapshot);
  preview.classList.remove('empty');
  preview.textContent = [
    `${origin}: ${stats.windows} window${stats.windows === 1 ? '' : 's'}, ${stats.tabs} tab${stats.tabs === 1 ? '' : 's'}, ${stats.groups} group${stats.groups === 1 ? '' : 's'}`,
    `Source: ${snapshot.source.browser}${snapshot.source.browserVersion === undefined ? '' : ` ${snapshot.source.browserVersion}`} on ${snapshot.source.platform}`,
    `Captured: ${new Date(snapshot.createdAt).toLocaleString()}`,
  ].join('\n');
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

function downloadSnapshot(bytes: Uint8Array, snapshot: TabSnapSnapshot): void {
  const blob = new Blob([bytesBuffer(bytes)], { type: 'application/octet-stream' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  const stamp = new Date(snapshot.createdAt).toISOString().replaceAll(':', '-');
  anchor.href = url;
  anchor.download = `tabsnap-${stamp}.tabsnap`;
  anchor.style.display = 'none';
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 0);
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
    setStatus('Encrypted string validated and decrypted locally. Review the preview before restore.', 'success');
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
      setStatus('Encrypted file validated and decrypted locally. Review the preview before restore.', 'success');
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

syncButtons();
