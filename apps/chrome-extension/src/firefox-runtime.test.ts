import { describe, expect, it } from 'vitest';

import { detectFirefoxRuntime, isFirefoxUserAgent } from './firefox-runtime.js';

const FIREFOX_USER_AGENT =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:145.0) Gecko/20100101 Firefox/145.0';

describe('Firefox runtime detection', () => {
  it('detects Firefox and its version', () => {
    expect(isFirefoxUserAgent(FIREFOX_USER_AGENT)).toBe(true);
    expect(detectFirefoxRuntime(FIREFOX_USER_AGENT)).toEqual({
      browser: 'firefox',
      displayName: 'Firefox',
      version: '145.0',
    });
  });

  it('does not treat Chromium user agents as Firefox', () => {
    expect(
      isFirefoxUserAgent(
        'Mozilla/5.0 AppleWebKit/537.36 Chrome/140.0.0.0 Safari/537.36 Edg/140.0.3485.54',
      ),
    ).toBe(false);
  });

  it('keeps Firefox metadata valid when the version token is unavailable', () => {
    expect(detectFirefoxRuntime('Mozilla/5.0 Gecko/20100101')).toEqual({
      browser: 'firefox',
      displayName: 'Firefox',
    });
  });
});
