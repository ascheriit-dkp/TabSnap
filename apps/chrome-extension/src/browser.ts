import type { Browser, TabSnapSnapshot } from '@tabsnap/schema';

import { chromiumAdapter } from './chromium-adapter.js';
import { detectChromiumRuntime } from './chromium-runtime.js';
import { firefoxAdapter } from './firefox-adapter.js';
import { isFirefoxUserAgent } from './firefox-runtime.js';
import type { BrowserAdapter, RestoreReport } from './webextension-adapter.js';

export function currentBrowser(userAgent = navigator.userAgent): Browser {
  if (isFirefoxUserAgent(userAgent)) return 'firefox';
  return detectChromiumRuntime(userAgent).browser;
}

function currentAdapter(userAgent = navigator.userAgent): BrowserAdapter {
  return currentBrowser(userAgent) === 'firefox' ? firefoxAdapter : chromiumAdapter;
}

export async function captureWorkspace(): Promise<TabSnapSnapshot> {
  return currentAdapter().captureWorkspace();
}

export async function restoreWorkspace(input: unknown): Promise<RestoreReport> {
  return currentAdapter().restoreWorkspace(input);
}

export type { RestoreReport } from './webextension-adapter.js';
