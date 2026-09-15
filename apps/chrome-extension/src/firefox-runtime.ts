import type { BrowserRuntime } from './webextension-adapter.js';

export function isFirefoxUserAgent(userAgent: string): boolean {
  return /\bFirefox\/[0-9.]+/u.test(userAgent);
}

export function detectFirefoxRuntime(userAgent: string): BrowserRuntime {
  const version = /\bFirefox\/([0-9.]+)/u.exec(userAgent)?.[1];
  return {
    browser: 'firefox',
    displayName: 'Firefox',
    ...(version !== undefined ? { version } : {}),
  };
}
