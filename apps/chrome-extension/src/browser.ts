import type { TabSnapSnapshot } from '@tabsnap/schema';

import { chromiumAdapter } from './chromium-adapter.js';
import { firefoxAdapter } from './firefox-adapter.js';
import { isFirefoxUserAgent } from './firefox-runtime.js';
import type { BrowserAdapter, RestoreReport } from './webextension-adapter.js';

function currentAdapter(userAgent = navigator.userAgent): BrowserAdapter {
  return isFirefoxUserAgent(userAgent) ? firefoxAdapter : chromiumAdapter;
}

export async function captureWorkspace(): Promise<TabSnapSnapshot> {
  return currentAdapter().captureWorkspace();
}

export async function restoreWorkspace(input: unknown): Promise<RestoreReport> {
  return currentAdapter().restoreWorkspace(input);
}

export type { RestoreReport } from './webextension-adapter.js';
