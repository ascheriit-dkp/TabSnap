import { createServer, type Server } from 'node:http';
import { type AddressInfo } from 'node:net';
import { resolve } from 'node:path';

import { chromium, expect, test, type BrowserContext, type Page } from '@playwright/test';

interface RestoredWorkspaceState {
  windowId: number;
  firstPinned: boolean;
  firstActive: boolean;
  groupTitle: string | null;
  groupColor: string | null;
  groupCollapsed: boolean | null;
}

interface WorkspaceInspection {
  restored: RestoredWorkspaceState | null;
  windows: Array<{
    id: number | null;
    focused: boolean;
    tabs: Array<{
      url: string | null;
      pendingUrl: string | null;
      status: string | null;
      pinned: boolean;
      active: boolean;
      groupId: number;
    }>;
  }>;
}

function hasExpectedRestore(inspection: WorkspaceInspection): boolean {
  const restored = inspection.restored;
  return (
    restored !== null &&
    restored.firstPinned &&
    restored.firstActive &&
    restored.groupTitle === 'Work' &&
    restored.groupColor === 'blue' &&
    restored.groupCollapsed === true
  );
}

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

async function inspectWorkspace(
  page: Page,
  firstUrl: string,
  secondUrl: string,
  originalWindowId: number,
): Promise<WorkspaceInspection> {
  return page.evaluate(
    async ({ expectedFirstUrl, expectedSecondUrl, excludedWindowId }) => {
      const tabUrl = (tab: chrome.tabs.Tab) => tab.url ?? tab.pendingUrl;
      const windows = await chrome.windows.getAll({
        populate: true,
        windowTypes: ['normal'],
      });
      const candidate = windows.find((window) => {
        if (window.id === excludedWindowId) return false;
        const urls = new Set((window.tabs ?? []).map(tabUrl));
        return urls.has(expectedFirstUrl) && urls.has(expectedSecondUrl);
      });

      let restored: RestoredWorkspaceState | null = null;
      if (candidate?.id !== undefined) {
        const firstTab = candidate.tabs?.find((tab) => tabUrl(tab) === expectedFirstUrl);
        const secondTab = candidate.tabs?.find((tab) => tabUrl(tab) === expectedSecondUrl);

        if (firstTab !== undefined && secondTab !== undefined) {
          const group =
            secondTab.groupId === chrome.tabGroups.TAB_GROUP_ID_NONE
              ? undefined
              : await chrome.tabGroups.get(secondTab.groupId);

          restored = {
            windowId: candidate.id,
            firstPinned: firstTab.pinned,
            firstActive: firstTab.active,
            groupTitle: group?.title ?? null,
            groupColor: group?.color ?? null,
            groupCollapsed: group?.collapsed ?? null,
          };
        }
      }

      return {
        restored,
        windows: windows.map((window) => ({
          id: window.id ?? null,
          focused: window.focused,
          tabs: (window.tabs ?? []).map((tab) => ({
            url: tab.url ?? null,
            pendingUrl: tab.pendingUrl ?? null,
            status: tab.status ?? null,
            pinned: tab.pinned,
            active: tab.active,
            groupId: tab.groupId,
          })),
        })),
      };
    },
    {
      expectedFirstUrl: firstUrl,
      expectedSecondUrl: secondUrl,
      excludedWindowId: originalWindowId,
    },
  );
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
          chrome.tabs.group(
            { tabIds: [secondTab.id!], createProperties: { windowId } },
            (createdGroupId) => {
              const error = chrome.runtime.lastError;
              if (error !== undefined) rejectGroup(new Error(error.message));
              else resolveGroup(createdGroupId);
            },
          );
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
        extensionPage.evaluate(
          async ({ firstUrl, secondUrl }) => {
            const tabs = await chrome.tabs.query({});
            return [firstUrl, secondUrl].every((url) =>
              tabs.some((tab) => (tab.url ?? tab.pendingUrl) === url && tab.status === 'complete'),
            );
          },
          { firstUrl: urlA, secondUrl: urlB },
        ),
      )
      .toBe(true);

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

    let inspection = await inspectWorkspace(extensionPage, urlA, urlB, originalWorkspace.windowId);
    for (let attempt = 0; attempt < 40 && !hasExpectedRestore(inspection); attempt += 1) {
      await extensionPage.waitForTimeout(250);
      inspection = await inspectWorkspace(extensionPage, urlA, urlB, originalWorkspace.windowId);
    }

    if (!hasExpectedRestore(inspection)) {
      throw new Error(`Restored workspace mismatch:\n${JSON.stringify(inspection, null, 2)}`);
    }

    expect(inspection.restored).not.toBeNull();
    expect(inspection.restored?.windowId).not.toBe(originalWorkspace.windowId);

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
