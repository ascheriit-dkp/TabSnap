import { describe, expect, it } from 'vitest';

import { normalizeChromiumWindowState, resolveChromiumRestoreUrl } from './chromium-adapter.js';
import { normalizeFirefoxWindowState, resolveFirefoxRestoreUrl } from './firefox-adapter.js';

describe('Chromium restore URL filtering', () => {
  it.each([
    'https://example.com/path?q=1',
    'http://localhost:3000/',
    'file:///C:/Users/example/notes.html',
    'about:blank',
    'chrome://newtab/',
    'edge://newtab/',
  ])('allows Chromium to attempt %s', (url) => {
    expect(resolveChromiumRestoreUrl(url)).toEqual({ attemptable: true, createUrl: url });
  });

  it.each([
    'javascript:alert(1)',
    'data:text/html,hello',
    'blob:https://example.com/id',
    'chrome-extension://abc/page.html',
    'edge-extension://abc/page.html',
    'devtools://devtools/bundled/inspector.html',
    'not a url',
  ])('blocks %s', (url) => {
    expect(resolveChromiumRestoreUrl(url)).toEqual({ attemptable: false });
  });

  it('normalizes locked fullscreen', () => {
    expect(normalizeChromiumWindowState('locked-fullscreen')).toBe('fullscreen');
  });
});

describe('Firefox restore URL filtering', () => {
  it.each(['https://example.com/', 'http://localhost:3000/', 'about:blank'])(
    'allows Firefox to attempt %s',
    (url) => {
      expect(resolveFirefoxRestoreUrl(url)).toEqual({ attemptable: true, createUrl: url });
    },
  );

  it('restores about:newtab by omitting the explicit URL', () => {
    expect(resolveFirefoxRestoreUrl('about:newtab')).toEqual({ attemptable: true });
  });

  it.each([
    'file:///tmp/local.html',
    'about:config',
    'about:addons',
    'about:debugging',
    'chrome://newtab/',
    'javascript:alert(1)',
    'data:text/html,hello',
    'blob:https://example.com/id',
    'moz-extension://abc/page.html',
    'not a url',
  ])('blocks Firefox restore for %s', (url) => {
    expect(resolveFirefoxRestoreUrl(url)).toEqual({ attemptable: false });
  });

  it('normalizes Firefox docked state without changing the snapshot schema', () => {
    expect(normalizeFirefoxWindowState('docked')).toBe('normal');
    expect(normalizeFirefoxWindowState('fullscreen')).toBe('fullscreen');
  });
});
