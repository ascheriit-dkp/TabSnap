import {
  parseTabSnapSnapshot,
  type Bounds,
  type Platform,
  type Tab as SnapshotTab,
  type TabGroup as SnapshotGroup,
  type TabSnapSnapshot,
  type WindowSnapshot,
  type WindowState,
} from '@tabsnap/schema';

const BLOCKED_RESTORE_PROTOCOLS = new Set([
  'blob:',
  'chrome-extension:',
  'data:',
  'devtools:',
  'filesystem:',
  'javascript:',
]);

const CHROME_GROUP_COLORS = new Set([
  'grey',
  'blue',
  'red',
  'yellow',
  'green',
  'pink',
  'purple',
  'cyan',
  'orange',
]);

export interface RestoreReport {
  createdWindows: number;
  createdTabs: number;
  skippedTabs: number;
  warnings: string[];
}

function platformFromChrome(os: chrome.runtime.PlatformOs): Platform {
  switch (os) {
    case 'win':
      return 'windows';
    case 'mac':
      return 'macos';
    case 'linux':
      return 'linux';
    default:
      throw new Error(`Unsupported platform for snapshot capture: ${os}.`);
  }
}

function snapshotWindowState(state: chrome.windows.WindowState | undefined): WindowState {
  switch (state) {
    case 'maximized':
    case 'minimized':
    case 'fullscreen':
    case 'normal':
      return state;
    default:
      return 'normal';
  }
}

function snapshotBounds(window: chrome.windows.Window): Bounds | undefined {
  const { left, top, width, height } = window;
  if (
    left === undefined ||
    top === undefined ||
    width === undefined ||
    height === undefined ||
    width <= 0 ||
    height <= 0
  ) {
    return undefined;
  }

  return {
    left: Math.trunc(left),
    top: Math.trunc(top),
    width: Math.trunc(width),
    height: Math.trunc(height),
  };
}

function tabUrl(tab: chrome.tabs.Tab): string {
  return tab.url ?? tab.pendingUrl ?? 'about:blank';
}

function browserVersion(): string | undefined {
  return /(?:Chrome|Chromium)\/([0-9.]+)/u.exec(navigator.userAgent)?.[1];
}

async function captureWindow(window: chrome.windows.Window, order: number): Promise<WindowSnapshot> {
  if (window.id === undefined) throw new Error('Chrome returned a window without an ID.');

  const tabs = [...(window.tabs ?? [])].sort((left, right) => left.index - right.index);
  if (tabs.length === 0) throw new Error('Chrome returned an empty normal window.');

  const groupPositions = new Map<number, number>();
  tabs.forEach((tab, tabOrder) => {
    if (tab.groupId !== chrome.tabGroups.TAB_GROUP_ID_NONE && !groupPositions.has(tab.groupId)) {
      groupPositions.set(tab.groupId, tabOrder);
    }
  });

  const chromeGroups = (await chrome.tabGroups.query({ windowId: window.id }))
    .filter((group) => groupPositions.has(group.id))
    .sort((left, right) => {
      const leftPosition = groupPositions.get(left.id) ?? Number.MAX_SAFE_INTEGER;
      const rightPosition = groupPositions.get(right.id) ?? Number.MAX_SAFE_INTEGER;
      return leftPosition - rightPosition || left.id - right.id;
    });

  const groupIdByChromeId = new Map<number, string>();
  const groups: SnapshotGroup[] = chromeGroups.map((group, groupOrder) => {
    const id = `group-${groupOrder}`;
    groupIdByChromeId.set(group.id, id);

    return {
      id,
      order: groupOrder,
      collapsed: group.collapsed,
      color: group.color,
      ...(group.title !== undefined ? { title: group.title } : {}),
    };
  });

  const snapshotTabs: SnapshotTab[] = tabs.map((tab, tabOrder) => {
    const canonicalGroupId = groupIdByChromeId.get(tab.groupId);
    return {
      order: tabOrder,
      url: tabUrl(tab),
      pinned: tab.pinned,
      active: tab.active,
      ...(tab.title !== undefined ? { title: tab.title } : {}),
      ...(canonicalGroupId !== undefined ? { groupId: canonicalGroupId } : {}),
    };
  });

  const bounds = snapshotBounds(window);
  return {
    order,
    state: snapshotWindowState(window.state),
    focused: window.focused,
    groups,
    tabs: snapshotTabs,
    ...(bounds !== undefined ? { bounds } : {}),
  };
}

export async function captureWorkspace(): Promise<TabSnapSnapshot> {
  const [windows, platformInfo] = await Promise.all([
    chrome.windows.getAll({ populate: true, windowTypes: ['normal'] }),
    chrome.runtime.getPlatformInfo(),
  ]);

  if (windows.length === 0) throw new Error('No normal Chrome windows are available to capture.');

  const version = browserVersion();
  const snapshot: TabSnapSnapshot = {
    format: 'tabsnap',
    formatVersion: 1,
    createdAt: new Date().toISOString(),
    source: {
      browser: 'chrome',
      platform: platformFromChrome(platformInfo.os),
      ...(version !== undefined ? { browserVersion: version } : {}),
    },
    windows: await Promise.all(windows.map((window, order) => captureWindow(window, order))),
  };

  return parseTabSnapSnapshot(snapshot);
}

export function isAttemptableRestoreUrl(value: string): boolean {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return false;
  }

  return !BLOCKED_RESTORE_PROTOCOLS.has(url.protocol);
}

function chromeGroupColor(value: string | undefined): chrome.tabGroups.Color | undefined {
  if (value === undefined || !CHROME_GROUP_COLORS.has(value)) return undefined;
  return value as chrome.tabGroups.Color;
}

async function restoreWindow(
  window: WindowSnapshot,
  report: RestoreReport,
): Promise<number | undefined> {
  const createdWindow = await chrome.windows.create({
    url: 'about:blank',
    focused: false,
    ...(window.bounds ?? {}),
  });

  if (createdWindow?.id === undefined) {
    report.warnings.push(`Could not create window ${window.order}.`);
    return undefined;
  }

  report.createdWindows += 1;
  const windowId = createdWindow.id;
  const bootstrapTabId = createdWindow.tabs?.[0]?.id;
  const createdTabIdsByOrder = new Map<number, number>();
  const tabs = [...window.tabs].sort((left, right) => left.order - right.order);

  for (const tab of tabs) {
    if (!isAttemptableRestoreUrl(tab.url)) {
      report.skippedTabs += 1;
      report.warnings.push(`Skipped unsupported URL: ${tab.url}`);
      continue;
    }

    try {
      const createdTab = await chrome.tabs.create({
        windowId,
        url: tab.url,
        active: false,
        pinned: false,
      });

      if (createdTab.id === undefined) throw new Error('Chrome returned a tab without an ID.');
      createdTabIdsByOrder.set(tab.order, createdTab.id);
      report.createdTabs += 1;
    } catch (error) {
      report.skippedTabs += 1;
      report.warnings.push(
        `Skipped ${tab.url}: ${error instanceof Error ? error.message : 'Chrome rejected the tab.'}`,
      );
    }
  }

  if (createdTabIdsByOrder.size > 0 && bootstrapTabId !== undefined) {
    try {
      await chrome.tabs.remove(bootstrapTabId);
    } catch (error) {
      report.warnings.push(
        `Could not remove bootstrap tab: ${error instanceof Error ? error.message : 'unknown error'}`,
      );
    }
  }

  const groups = [...window.groups].sort((left, right) => left.order - right.order);
  for (const group of groups) {
    const tabIds = tabs
      .filter((tab) => tab.groupId === group.id)
      .map((tab) => createdTabIdsByOrder.get(tab.order))
      .filter((tabId): tabId is number => tabId !== undefined);

    if (tabIds.length === 0) continue;

    try {
      const groupId = await chrome.tabs.group({ tabIds, createProperties: { windowId } });
      const color = chromeGroupColor(group.color);
      await chrome.tabGroups.update(groupId, {
        collapsed: group.collapsed,
        ...(group.title !== undefined ? { title: group.title } : {}),
        ...(color !== undefined ? { color } : {}),
      });
    } catch (error) {
      report.warnings.push(
        `Could not restore group ${group.title ?? group.id}: ${
          error instanceof Error ? error.message : 'unknown error'
        }`,
      );
    }
  }

  for (const tab of tabs) {
    if (!tab.pinned) continue;
    const tabId = createdTabIdsByOrder.get(tab.order);
    if (tabId === undefined) continue;

    if (tab.groupId !== undefined) {
      report.warnings.push(`Ignored impossible pinned+grouped state for ${tab.url}.`);
      continue;
    }

    try {
      await chrome.tabs.update(tabId, { pinned: true });
    } catch (error) {
      report.warnings.push(
        `Could not pin ${tab.url}: ${error instanceof Error ? error.message : 'unknown error'}`,
      );
    }
  }

  const requestedActive = tabs.find((tab) => tab.active);
  const activeTabId =
    (requestedActive === undefined ? undefined : createdTabIdsByOrder.get(requestedActive.order)) ??
    createdTabIdsByOrder.values().next().value;

  if (activeTabId !== undefined) {
    try {
      await chrome.tabs.update(activeTabId, { active: true });
    } catch (error) {
      report.warnings.push(
        `Could not activate restored tab: ${error instanceof Error ? error.message : 'unknown error'}`,
      );
    }
  }

  if (window.state !== 'normal') {
    try {
      await chrome.windows.update(windowId, { state: window.state });
    } catch (error) {
      report.warnings.push(
        `Could not restore window state ${window.state}: ${
          error instanceof Error ? error.message : 'unknown error'
        }`,
      );
    }
  }

  return windowId;
}

export async function restoreWorkspace(input: unknown): Promise<RestoreReport> {
  const snapshot = parseTabSnapSnapshot(input);
  const report: RestoreReport = {
    createdWindows: 0,
    createdTabs: 0,
    skippedTabs: 0,
    warnings: [],
  };

  let focusedWindowId: number | undefined;
  const windows = [...snapshot.windows].sort((left, right) => left.order - right.order);

  for (const window of windows) {
    try {
      const windowId = await restoreWindow(window, report);
      if (window.focused && windowId !== undefined) focusedWindowId = windowId;
    } catch (error) {
      report.warnings.push(
        `Could not restore window ${window.order}: ${
          error instanceof Error ? error.message : 'unknown error'
        }`,
      );
    }
  }

  if (focusedWindowId !== undefined) {
    try {
      await chrome.windows.update(focusedWindowId, { focused: true });
    } catch (error) {
      report.warnings.push(
        `Could not focus restored window: ${error instanceof Error ? error.message : 'unknown error'}`,
      );
    }
  }

  return report;
}
