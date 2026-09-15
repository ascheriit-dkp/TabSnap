import { createServer, type Server } from 'node:http';
import { type AddressInfo } from 'node:net';
import { resolve } from 'node:path';

import { chromium, expect, test, type BrowserContext, type Page } from '@playwright/test';

async function listen(server: Server): Promise<number> {
  await new Promise<void>((resolveListen, reject) => {
    const onError = (error: Error) => reject(error);
    server.once('error', onError);
    server.listen(0, '127.0.0.1', () => {
      server.off('error', onError);
      resolveListen();
    });
  });

  const address = server.address();
  if (address === null || typeof address === 'string') {
    throw new Error('Local test server did not expose an IPv4 address.');
  }

  return (address as AddressInfo).port;
}

async function closeServer(server: Server): Promise<void> {
  await new Promise<void>((resolveClose, reject) => {
    server.close((error) => {
      if (error) reject(error);
      else resolveClose();
    });
  });
}

async function readTabSnapExtensionId(page: Page): Promise<string | null> {
  return page.evaluate(() => {
    const roots: Array<Document | ShadowRoot> = [document];

    while (roots.length > 0) {
      const root = roots.shift();
      if (root === undefined) break;

      for (const element of root.querySelectorAll('*')) {
        if (element.shadowRoot !== null) roots.push(element.shadowRoot);
        if (element.tagName.toLowerCase() !== 'extensions-item') continue;

        const name = element.shadowRoot?.querySelector('#name')?.textContent?.trim();
        if (name === 'TabSnap') return element.id || element.getAttribute('id');
      }
    }

    return null;
  });
}

async function openExtensionPage(context: BrowserContext): Promise<Page> {
  const page = context.pages()[0] ?? (await context.newPage());
  await page.goto('chrome://extensions/');

  await expect
    .poll(() => readTabSnapExtensionId(page), {
      message: 'TabSnap should be loaded as an unpacked Chromium extension.',
      timeout: 10_000,
    })
    .not.toBeNull();

  const extensionId = await readTabSnapExtensionId(page);
  if (extensionId === null) throw new Error('Could not resolve the TabSnap extension ID.');

  await page.goto(`chrome-extension://${extensionId}/index.html`);
  return page;
}

test('captures and restores a real Chrome workspace non-destructively', async () => {
  const server = createServer((request, response) => {
    response.writeHead(200, {
      'content-type': 'text/html; charset=utf-8',
      'cache-control': 'no-store',
    });
    response.end(`<!doctype html><title>TabSnap fixture</title><h1>${request.url ?? '/'}</h1>`);
  });
  const port = await listen(server);
  const urlA = `http://127.0.0.1:${port}/alpha`;
  const urlB = `http://127.0.0.1:${port}/beta`;
  const extensionPath = resolve('apps/chrome-extension/dist');

  const context = await chromium.launchPersistentContext('', {
    channel: 'chromium',
    headless: true,
    args: [`--disable-extensions-except=${extensionPath}`, `--load-extension=${extensionPath}`],
  });

  try {
    const extensionPage = await openExtensionPage(context);
    await expect(extensionPage.locator('h1')).toHaveText('TabSnap');
    await expect(extensionPage.locator('.local-badge')).toHaveText('No backend');

    const granted = await extensionPage.evaluate(
      () =>
        new Promise<chrome.permissions.Permissions>((resolvePermissions) => {
          chrome.permissions.getAll(resolvePermissions);
        }),
    );
    expect([...(granted.permissions ?? [])].sort()).toEqual(['tabGroups', 'tabs']);
    expect(granted.origins ?? []).toEqual([]);

    const originalWorkspace = await extensionPage.evaluate(
      async ({ firstUrl, secondUrl }) => {
        const testWindow = await chrome.windows.create({ url: firstUrl, focused: false });
        const windowId = testWindow?.id;
        const firstTabId = testWindow?.tabs?.[0]?.id;
        if (windowId === undefined || firstTabId === undefined) {
          throw new Error('Chromium did not create the fixture window and first tab.');
        }

        await chrome.tabs.update(firstTabId, { pinned: true, active: true });
        const secondTab = await chrome.tabs.create({
          windowId,
          url: secondUrl,
          active: false,
        });
        if (secondTab.id === undefined) throw new Error('Chromium did not create the second tab.');

        const groupId = await new Promise<number>((resolveGroup, rejectGroup) => {
          chrome.tabs.group({ tabIds: [secondTab.id!] }, (createdGroupId) => {
            const error = chrome.runtime.lastError;
            if (error !== undefined) rejectGroup(new Error(error.message));
            else resolveGroup(createdGroupId);
          });
        });
        await chrome.tabGroups.update(groupId, {
          title: 'Work',
          color: 'blue',
          collapsed: true,
        });

        return { windowId, firstTabId, secondTabId: secondTab.id };
      },
      { firstUrl: urlA, secondUrl: urlB },
    );

    await expect
      .poll(() =>
        extensionPage.evaluate(async () => {
          const windows = await chrome.windows.getAll({ windowTypes: ['normal'] });
          return windows.length;
        }),
      )
      .toBe(2);

    await extensionPage.locator('#capture').click();
    await expect(extensionPage.locator('#status')).toHaveText(
      'Workspace captured locally. Nothing has been uploaded.',
    );
    await expect(extensionPage.locator('#preview')).toContainText(
      'Captured: 2 windows, 3 tabs, 1 group',
    );

    await extensionPage.locator('#restore').click();
    await expect(extensionPage.locator('#status')).toContainText('Restored 2 windows and 2 tabs.');
    await expect(extensionPage.locator('#status')).toContainText('1 tab skipped.');

    await expect
      .poll(() =>
        extensionPage.evaluate(async () => {
          const windows = await chrome.windows.getAll({ windowTypes: ['normal'] });
          return windows.length;
        }),
      )
      .toBe(4);

    const restored = await extensionPage.evaluate(
      async ({ firstUrl, secondUrl, originalWindowId }) => {
        const windows = await chrome.windows.getAll({
          populate: true,
          windowTypes: ['normal'],
        });
        const candidate = windows.find((window) => {
          if (window.id === originalWindowId) return false;
          const urls = new Set((window.tabs ?? []).map((tab) => tab.url));
          return urls.has(firstUrl) && urls.has(secondUrl);
        });
        if (candidate?.id === undefined) return null;

        const firstTab = candidate.tabs?.find((tab) => tab.url === firstUrl);
        const secondTab = candidate.tabs?.find((tab) => tab.url === secondUrl);
        if (firstTab === undefined || secondTab === undefined) return null;
        if (secondTab.groupId === chrome.tabGroups.TAB_GROUP_ID_NONE) return null;

        const group = await chrome.tabGroups.get(secondTab.groupId);
        return {
          windowId: candidate.id,
          firstPinned: firstTab.pinned,
          firstActive: firstTab.active,
          groupTitle: group.title,
          groupColor: group.color,
          groupCollapsed: group.collapsed,
        };
      },
      {
        firstUrl: urlA,
        secondUrl: urlB,
        originalWindowId: originalWorkspace.windowId,
      },
    );

    expect(restored).not.toBeNull();
    expect(restored?.windowId).not.toBe(originalWorkspace.windowId);
    expect(restored?.firstPinned).toBe(true);
    expect(restored?.firstActive).toBe(true);
    expect(restored?.groupTitle).toBe('Work');
    expect(restored?.groupColor).toBe('blue');
    expect(restored?.groupCollapsed).toBe(true);

    const originalStillExists = await extensionPage.evaluate(async (windowId) => {
      try {
        await chrome.windows.get(windowId);
        return true;
      } catch {
        return false;
      }
    }, originalWorkspace.windowId);
    expect(originalStillExists).toBe(true);
  } finally {
    await context.close();
    await closeServer(server);
  }
});
