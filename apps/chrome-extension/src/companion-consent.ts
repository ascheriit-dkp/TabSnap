import type { Browser } from '@tabsnap/schema';

export const FIREFOX_COMPANION_DATA_PERMISSION = 'browsingActivity';

interface FirefoxDataCollectionPermissionsApi {
  request(
    permissions: { data_collection: string[] },
    callback: (granted: boolean) => void,
  ): void;
}

export async function requestCompanionDataConsent(
  browser: Browser,
  permissionsApi: FirefoxDataCollectionPermissionsApi = chrome.permissions as unknown as FirefoxDataCollectionPermissionsApi,
): Promise<boolean> {
  if (browser !== 'firefox') return true;

  return new Promise((resolvePermission) => {
    permissionsApi.request(
      { data_collection: [FIREFOX_COMPANION_DATA_PERMISSION] },
      resolvePermission,
    );
  });
}
