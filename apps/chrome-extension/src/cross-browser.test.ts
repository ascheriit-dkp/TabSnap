import type { Browser, TabSnapSnapshot } from '@tabsnap/schema';
import { describe, expect, it } from 'vitest';

import { assessCrossBrowserCompatibility } from './cross-browser.js';

function snapshot(source: Browser, urls: string[]): TabSnapSnapshot {
  return {
    format: 'tabsnap',
    formatVersion: 1,
    createdAt: '2026-09-16T00:00:00.000Z',
    source: {
      browser: source,
      platform: 'windows',
    },
    windows: [
      {
        order: 0,
        state: 'normal',
        focused: true,
        groups: [],
        tabs: urls.map((url, order) => ({
          order,
          url,
          pinned: false,
          active: order === 0,
        })),
      },
    ],
  };
}

describe('cross-browser compatibility', () => {
  it('does not warn for a same-browser restore', () => {
    expect(
      assessCrossBrowserCompatibility(snapshot('chrome', ['https://example.com/']), 'chrome'),
    ).toEqual({
      crossBrowser: false,
      source: 'chrome',
      target: 'chrome',
      knownNonPortableTabs: 0,
      notes: [],
    });
  });

  it('keeps ordinary Chrome to Edge workspaces portable and flags Chrome internals', () => {
    const report = assessCrossBrowserCompatibility(
      snapshot('chrome', ['https://example.com/', 'chrome://settings/']),
      'edge',
    );

    expect(report.crossBrowser).toBe(true);
    expect(report.knownNonPortableTabs).toBe(1);
    expect(report.notes).toHaveLength(1);
  });

  it('flags Edge internals when restoring into Chrome', () => {
    const report = assessCrossBrowserCompatibility(
      snapshot('edge', ['https://example.com/', 'edge://settings/']),
      'chrome',
    );

    expect(report.knownNonPortableTabs).toBe(1);
  });

  it('uses the Firefox restore policy for Chromium to Firefox moves', () => {
    const report = assessCrossBrowserCompatibility(
      snapshot('chrome', [
        'https://example.com/',
        'file:///C:/Users/example/notes.html',
        'chrome://newtab/',
        'about:blank',
      ]),
      'firefox',
    );

    expect(report.knownNonPortableTabs).toBe(2);
    expect(report.notes).toHaveLength(2);
  });

  it('flags Firefox-only privileged pages when restoring into Chromium', () => {
    const report = assessCrossBrowserCompatibility(
      snapshot('firefox', [
        'https://example.com/',
        'about:config',
        'moz-extension://abc/page.html',
        'about:newtab',
      ]),
      'chrome',
    );

    expect(report.knownNonPortableTabs).toBe(2);
  });
});
