import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { type AddressInfo } from 'node:net';
import { resolve } from 'node:path';

import { chromium, expect, test, type BrowserContext, type Page } from '@playwright/test';

const EDGE_USER_AGENT =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.3485.54';
const SESSION_TOKEN = 'a'.repeat(64);

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
    throw new Error('Local Edge compatibility server did not expose an IPv4 address.');
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

function corsHeaders(request: IncomingMessage): Record<string, string> {
  const origin = request.headers.origin ?? '';
  return {
    'access-control-allow-origin': origin,
    'access-control-allow-methods': 'GET, POST, OPTIONS',
    'access-control-allow-headers': 'Authorization, Content-Type, X-TabSnap-Name',
    'cache-control': 'no-store',
  };
}

function json(response: ServerResponse, request: IncomingMessage, value: unknown): void {
  response.writeHead(200, {
    ...corsHeaders(request),
    'content-type': 'application/json; charset=utf-8',
  });
  response.end(JSON.stringify(value));
}

function companionServer(): Server {
  return createServer((request, response) => {
    if (request.method === 'OPTIONS') {
      response.writeHead(204, corsHeaders(request));
      response.end();
      return;
    }

    if (request.headers.authorization !== `Bearer ${SESSION_TOKEN}`) {
      response.writeHead(401, { ...corsHeaders(request), 'content-type': 'application/json' });
      response.end(JSON.stringify({ error: 'unauthorized' }));
      return;
    }

    if (request.method === 'GET' && request.url === '/v1/status') {
      json(response, request, {
        protocolVersion: 1,
        transport: 'loopback-http',
        authentication: 'session-bearer',
      });
      return;
    }

    if (request.method === 'GET' && request.url === '/v1/snapshots') {
      json(response, request, { protocolVersion: 1, snapshots: [] });
      return;
    }

    response.writeHead(404, { ...corsHeaders(request), 'content-type': 'application/json' });
    response.end(JSON.stringify({ error: 'not found' }));
  });
}

test('detects Edge metadata and connects to the local companion in the shared Chromium build', async () => {
  const fixtureServer = createServer((request, response) => {
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end(`<!doctype html><title>Edge fixture</title><p>${request.url ?? '/'}</p>`);
  });
  const companion = companionServer();
  const fixturePort = await listen(fixtureServer);
  const companionPort = await listen(companion);
  const fixtureUrl = `http://127.0.0.1:${fixturePort}/edge`;
  const extensionPath = resolve('apps/chrome-extension/dist');

  const context = await chromium.launchPersistentContext('', {
    channel: 'chromium',
    headless: true,
    userAgent: EDGE_USER_AGENT,
    args: [`--disable-extensions-except=${extensionPath}`, `--load-extension=${extensionPath}`],
  });

  try {
    const extensionPage = await openExtensionPage(context);
    expect(await extensionPage.evaluate(() => navigator.userAgent)).toContain('Edg/140.0.3485.54');

    await extensionPage.evaluate(async (url) => {
      await chrome.windows.create({ url, focused: false });
    }, fixtureUrl);

    await expect
      .poll(() =>
        extensionPage.evaluate(async (url) => {
          const tabs = await chrome.tabs.query({});
          return tabs.some(
            (tab) => (tab.url ?? tab.pendingUrl) === url && tab.status === 'complete',
          );
        }, fixtureUrl),
      )
      .toBe(true);

    await extensionPage.locator('#capture').click();
    await expect(extensionPage.locator('#status')).toHaveText(
      'Workspace captured locally. Nothing has been uploaded.',
    );
    await expect(extensionPage.locator('#preview')).toContainText('Source: edge 140.0.3485.54');

    await extensionPage
      .locator('#companion-pairing')
      .fill(`tabsnap-companion:v1:${companionPort}:${SESSION_TOKEN}`);
    await extensionPage.locator('#companion-connect').click();

    await expect(extensionPage.locator('#companion-state')).toHaveText('Connected');
    await expect(extensionPage.locator('#status')).toContainText(
      'Portable companion connected for this page session.',
    );
    await expect(extensionPage.locator('#companion-snapshots')).toHaveText('Library is empty');

    const granted = await extensionPage.evaluate(
      () =>
        new Promise<chrome.permissions.Permissions>((resolvePermissions) => {
          chrome.permissions.getAll(resolvePermissions);
        }),
    );
    expect(granted.origins).toContain('http://127.0.0.1/*');
  } finally {
    await context.close();
    await closeServer(companion);
    await closeServer(fixtureServer);
  }
});
