import type { WindowState } from '@tabsnap/schema';

import { detectFirefoxRuntime } from './firefox-runtime.js';
import { createWebExtensionAdapter, type RestoreUrlDecision } from './webextension-adapter.js';

const BLOCKED_FIREFOX_PROTOCOLS = new Set([
  'blob:',
  'chrome:',
  'chrome-extension:',
  'data:',
  'devtools:',
  'edge:',
  'edge-extension:',
  'file:',
  'filesystem:',
  'javascript:',
  'moz-extension:',
  'resource:',
]);

export function resolveFirefoxRestoreUrl(value: string): RestoreUrlDecision {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return { attemptable: false };
  }

  if (BLOCKED_FIREFOX_PROTOCOLS.has(url.protocol)) return { attemptable: false };

  if (url.protocol === 'about:') {
    if (value === 'about:blank') return { attemptable: true, createUrl: value };
    if (value === 'about:newtab') return { attemptable: true };
    return { attemptable: false };
  }

  return { attemptable: true, createUrl: value };
}

export function normalizeFirefoxWindowState(state: string | undefined): WindowState {
  switch (state) {
    case 'maximized':
    case 'minimized':
    case 'fullscreen':
    case 'normal':
      return state;
    case 'docked':
    default:
      return 'normal';
  }
}

export const firefoxAdapter = createWebExtensionAdapter({
  detectRuntime: detectFirefoxRuntime,
  normalizeWindowState: normalizeFirefoxWindowState,
  resolveRestoreUrl: resolveFirefoxRestoreUrl,
});
