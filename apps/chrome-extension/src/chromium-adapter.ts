import type { WindowState } from '@tabsnap/schema';

import { detectChromiumRuntime } from './chromium-runtime.js';
import {
  createWebExtensionAdapter,
  type RestoreUrlDecision,
} from './webextension-adapter.js';

const BLOCKED_RESTORE_PROTOCOLS = new Set([
  'blob:',
  'chrome-extension:',
  'data:',
  'devtools:',
  'edge-extension:',
  'filesystem:',
  'javascript:',
]);

export function resolveChromiumRestoreUrl(value: string): RestoreUrlDecision {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return { attemptable: false };
  }

  if (BLOCKED_RESTORE_PROTOCOLS.has(url.protocol)) return { attemptable: false };
  return { attemptable: true, createUrl: value };
}

export function normalizeChromiumWindowState(state: string | undefined): WindowState {
  switch (state) {
    case 'maximized':
    case 'minimized':
    case 'fullscreen':
    case 'normal':
      return state;
    case 'locked-fullscreen':
      return 'fullscreen';
    default:
      return 'normal';
  }
}

export const chromiumAdapter = createWebExtensionAdapter({
  detectRuntime: detectChromiumRuntime,
  normalizeWindowState: normalizeChromiumWindowState,
  resolveRestoreUrl: resolveChromiumRestoreUrl,
});
