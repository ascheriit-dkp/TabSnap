import { describe, expect, it, vi } from 'vitest';

import {
  FIREFOX_COMPANION_DATA_PERMISSION,
  requestCompanionDataConsent,
} from './companion-consent.js';

describe('companion data consent', () => {
  it.each(['chrome', 'edge'] as const)('does not request Mozilla data consent in %s', async (browser) => {
    const request = vi.fn();

    await expect(requestCompanionDataConsent(browser, { request })).resolves.toBe(true);
    expect(request).not.toHaveBeenCalled();
  });

  it('requests optional browsing activity consent in Firefox', async () => {
    const request = vi.fn((permissions, callback: (granted: boolean) => void) => {
      expect(permissions).toEqual({ data_collection: [FIREFOX_COMPANION_DATA_PERMISSION] });
      callback(true);
    });

    await expect(requestCompanionDataConsent('firefox', { request })).resolves.toBe(true);
    expect(request).toHaveBeenCalledOnce();
  });

  it('keeps companion mode off when Firefox consent is declined', async () => {
    const request = vi.fn((_permissions, callback: (granted: boolean) => void) => callback(false));

    await expect(requestCompanionDataConsent('firefox', { request })).resolves.toBe(false);
  });
});
