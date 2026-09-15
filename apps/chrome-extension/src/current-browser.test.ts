import { describe, expect, it } from 'vitest';

import { currentBrowser } from './browser.js';

describe('current browser target', () => {
  it('detects Chrome', () => {
    expect(
      currentBrowser(
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/140.0.0.0 Safari/537.36',
      ),
    ).toBe('chrome');
  });

  it('detects Microsoft Edge before Chrome', () => {
    expect(
      currentBrowser(
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0',
      ),
    ).toBe('edge');
  });

  it('detects Firefox', () => {
    expect(
      currentBrowser(
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:143.0) Gecko/20100101 Firefox/143.0',
      ),
    ).toBe('firefox');
  });
});
