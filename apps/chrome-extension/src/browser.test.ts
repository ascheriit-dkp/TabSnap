import { describe, expect, it } from 'vitest';

import { isAttemptableRestoreUrl } from './browser.js';

describe('restore URL filtering', () => {
  it.each([
    'https://example.com/path?q=1',
    'http://localhost:3000/',
    'file:///C:/Users/example/notes.html',
    'about:blank',
    'chrome://newtab/',
  ])('allows Chrome to attempt %s', (url) => {
    expect(isAttemptableRestoreUrl(url)).toBe(true);
  });

  it.each([
    'javascript:alert(1)',
    'data:text/html,hello',
    'blob:https://example.com/id',
    'chrome-extension://abc/page.html',
    'devtools://devtools/bundled/inspector.html',
    'not a url',
  ])('blocks %s', (url) => {
    expect(isAttemptableRestoreUrl(url)).toBe(false);
  });
});
