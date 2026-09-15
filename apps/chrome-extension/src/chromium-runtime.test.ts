import { describe, expect, it } from 'vitest';

import { detectChromiumRuntime } from './chromium-runtime.js';

describe('Chromium runtime detection', () => {
  it('detects Microsoft Edge before the shared Chrome token', () => {
    expect(
      detectChromiumRuntime(
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.3485.54',
      ),
    ).toEqual({
      browser: 'edge',
      displayName: 'Microsoft Edge',
      version: '140.0.3485.54',
    });
  });

  it('detects Chrome', () => {
    expect(
      detectChromiumRuntime(
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.7339.81 Safari/537.36',
      ),
    ).toEqual({
      browser: 'chrome',
      displayName: 'Chrome',
      version: '140.0.7339.81',
    });
  });

  it('keeps Chromium-compatible builds on the Chrome schema value', () => {
    expect(detectChromiumRuntime('Mozilla/5.0 Chromium/140.0.0.0')).toEqual({
      browser: 'chrome',
      displayName: 'Chromium',
      version: '140.0.0.0',
    });
  });

  it('fails closed to the existing Chrome schema value when the UA is reduced beyond recognition', () => {
    expect(detectChromiumRuntime('Mozilla/5.0')).toEqual({
      browser: 'chrome',
      displayName: 'Chromium',
    });
  });
});
