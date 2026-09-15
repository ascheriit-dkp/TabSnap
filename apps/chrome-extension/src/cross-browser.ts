import type { Browser, TabSnapSnapshot } from '@tabsnap/schema';

import { resolveChromiumRestoreUrl } from './chromium-adapter.js';
import { resolveFirefoxRestoreUrl } from './firefox-adapter.js';

export interface CrossBrowserCompatibility {
  crossBrowser: boolean;
  source: Browser;
  target: Browser;
  knownNonPortableTabs: number;
  notes: string[];
}

function protocol(value: string): string | undefined {
  try {
    return new URL(value).protocol;
  } catch {
    return undefined;
  }
}

function isKnownNonPortableUrl(value: string, source: Browser, target: Browser): boolean {
  if (target === 'firefox') {
    return !resolveFirefoxRestoreUrl(value).attemptable;
  }

  if (!resolveChromiumRestoreUrl(value).attemptable) return true;

  const urlProtocol = protocol(value);
  if (source === 'chrome' && target === 'edge' && urlProtocol === 'chrome:') return true;
  if (source === 'edge' && target === 'chrome' && urlProtocol === 'edge:') return true;

  if (source === 'firefox') {
    if (urlProtocol === 'moz-extension:' || urlProtocol === 'resource:') return true;
    if (urlProtocol === 'about:' && value !== 'about:blank' && value !== 'about:newtab')
      return true;
  }

  return false;
}

export function assessCrossBrowserCompatibility(
  snapshot: TabSnapSnapshot,
  target: Browser,
): CrossBrowserCompatibility {
  const source = snapshot.source.browser;
  if (source === target) {
    return {
      crossBrowser: false,
      source,
      target,
      knownNonPortableTabs: 0,
      notes: [],
    };
  }

  const tabs = snapshot.windows.flatMap((window) => window.tabs);
  const knownNonPortableTabs = tabs.filter((tab) =>
    isKnownNonPortableUrl(tab.url, source, target),
  ).length;
  const notes: string[] = [];

  if ((source === 'chrome' && target === 'edge') || (source === 'edge' && target === 'chrome')) {
    notes.push(
      'Chrome and Edge share the same workspace model; browser-internal URLs are best effort.',
    );
  } else {
    notes.push(
      'Common web tabs, ordering, pinning, groups and window geometry are portable; browser-privileged URLs are best effort.',
    );
    notes.push(
      'Collapsed tab groups can look different in Firefox when the active tab belongs to the collapsed group.',
    );
  }

  return {
    crossBrowser: true,
    source,
    target,
    knownNonPortableTabs,
    notes,
  };
}
