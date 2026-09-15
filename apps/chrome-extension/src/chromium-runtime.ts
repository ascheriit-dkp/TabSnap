import type { Browser } from '@tabsnap/schema';

export interface ChromiumRuntime {
  browser: Extract<Browser, 'chrome' | 'edge'>;
  displayName: 'Chrome' | 'Chromium' | 'Microsoft Edge';
  version?: string;
}

function versionFromUserAgent(userAgent: string, token: RegExp): string | undefined {
  return token.exec(userAgent)?.[1];
}

export function detectChromiumRuntime(userAgent: string): ChromiumRuntime {
  const edgeVersion = versionFromUserAgent(userAgent, /\bEdg\/([0-9.]+)/u);
  if (edgeVersion !== undefined) {
    return {
      browser: 'edge',
      displayName: 'Microsoft Edge',
      version: edgeVersion,
    };
  }

  const chromeVersion = versionFromUserAgent(userAgent, /\bChrome\/([0-9.]+)/u);
  if (chromeVersion !== undefined) {
    return {
      browser: 'chrome',
      displayName: 'Chrome',
      version: chromeVersion,
    };
  }

  const chromiumVersion = versionFromUserAgent(userAgent, /\bChromium\/([0-9.]+)/u);
  return {
    browser: 'chrome',
    displayName: 'Chromium',
    ...(chromiumVersion !== undefined ? { version: chromiumVersion } : {}),
  };
}
