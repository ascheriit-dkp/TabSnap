import {
  parseTabSnapSnapshot,
  type Bounds,
  type Browser,
  type Platform,
  type Tab as SnapshotTab,
  type TabGroup as SnapshotGroup,
  type TabSnapSnapshot,
  type WindowSnapshot,
  type WindowState,
} from '@tabsnap/schema';

const SHARED_GROUP_COLORS = new Set([
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

export interface BrowserRuntime {
  browser: Browser;
  displayName: string;
  version?: string;
}

export interface RestoreUrlDecision {
  attemptable: boolean;
  createUrl?: string;
}

export interface BrowserAdapterConfig {
  detectRuntime(userAgent: string): BrowserRuntime;
  normalizeWindowState(state: string | undefined): WindowState;
  resolveRestoreUrl(value: string): RestoreUrlDecision;
}

export interface RestoreReport {
  createdWindows: number;
  createdTabs: number;
  skippedTabs: number;
  warnings: string[];
}

export interface BrowserAdapter {
  captureWorkspace(): Promise<TabSnapSnapshot>;
  restoreWorkspace(input: unknown): Promise<RestoreReport>;
}

function platformFromWebExtension(os: chrome.runtime.PlatformInfo['os']): Platform {
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

function groupColor(value: string | undefined): chrome.tabGroups.Color | undefined {
  if (value === undefined || !SHARED_GROUP_COLORS.has(value)) return undefined;
  return value as chrome.tabGroups.Color;
}

function groupTabs(tabIds: [number, ...number[]], windowId: number): Promise<number> {
  return new Promise((resolve, reject) => {
    chrome.tabs.group({ tabIds, createProperties: { windowId } }, (groupId) => {
      const error = chrome.runtime.lastError;
      if (error !== undefined) {
        reject(new Error(error.message));
        return;
      }
      resolve(groupId);
    });
  });
}

export function createWebExtensionAdapter(config: BrowserAdapterConfig): BrowserAdapter {
  async function captureWindow(
    window: chrome.windows.Window,
    order: number,
  ): Promise<WindowSnapshot> {
    if (window.id === undefined) throw new Error('Browser API returned a window without an ID.');

    const tabs = [...(window.tabs ?? [])].sort((left, right) => left.index - right.index);
    if (tabs.length === 0) throw new Error('Browser API returned an empty normal window.');

    const groupPositions = new Map<number, number>();
    tabs.forEach((tab, tabOrder) => {
      if (tab.groupId !== chrome.tabGroups.TAB_GROUP_ID_NONE && !groupPositions.has(tab.groupId)) {
        groupPositions.set(tab.groupId, tabOrder);
      }
    });

    const browserGroups = (await chrome.tabGroups.query({ windowId: window.id }))
      .filter((group) => groupPositions.has(group.id))
      .sort((left, right) => {
        const leftPosition = groupPositions.get(left.id) ?? Number.MAX_SAFE_INTEGER;
        const rightPosition = groupPositions.get(right.id) ?? Number.MAX_SAFE_INTEGER;
        return leftPosition - rightPosition || left.id - right.id;
      });

    const groupIdByBrowserId = new Map<number, string>();
    const groups: SnapshotGroup[] = browserGroups.map((group, groupOrder) => {
      const id = `group-${groupOrder}`;
      groupIdByBrowserId.set(group.id, id);

      return {
        id,
        order: groupOrder,
        collapsed: group.collapsed,
        color: group.color,
        ...(group.title !== undefined ? { title: group.title } : {}),
      };
    });

    const snapshotTabs: SnapshotTab[] = tabs.map((tab, tabOrder) => {
      const canonicalGroupId = groupIdByBrowserId.get(tab.groupId);
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
      state: config.normalizeWindowState(window.state),
      focused: window.focused,
      groups,
      tabs: snapshotTabs,
      ...(bounds !== undefined ? { bounds } : {}),
    };
  }

  async function captureWorkspace(): Promise<TabSnapSnapshot> {
    const runtime = config.detectRuntime(navigator.userAgent);
    const [windows, platformInfo] = await Promise.all([
      chrome.windows.getAll({ populate: true, windowTypes: ['normal'] }),
      chrome.runtime.getPlatformInfo(),
    ]);

    if (windows.length === 0) {
      throw new Error(`No normal ${runtime.displayName} windows are available to capture.`);
    }

    const snapshot: TabSnapSnapshot = {
      format: 'tabsnap',
      formatVersion: 1,
      createdAt: new Date().toISOString(),
      source: {
        browser: runtime.browser,
        platform: platformFromWebExtension(platformInfo.os),
        ...(runtime.version !== undefined ? { browserVersion: runtime.version } : {}),
      },
      windows: await Promise.all(windows.map((window, order) => captureWindow(window, order))),
    };

    return parseTabSnapSnapshot(snapshot);
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
      const decision = config.resolveRestoreUrl(tab.url);
      if (!decision.attemptable) {
        report.skippedTabs += 1;
        report.warnings.push(`Skipped unsupported URL: ${tab.url}`);
        continue;
      }

      try {
        const createdTab = await chrome.tabs.create({
          windowId,
          ...(decision.createUrl !== undefined ? { url: decision.createUrl } : {}),
          active: false,
          pinned: false,
        });

        if (createdTab.id === undefined) throw new Error('Browser API returned a tab without an ID.');
        createdTabIdsByOrder.set(tab.order, createdTab.id);
        report.createdTabs += 1;
      } catch (error) {
        report.skippedTabs += 1;
        report.warnings.push(
          `Skipped ${tab.url}: ${error instanceof Error ? error.message : 'browser rejected the tab.'}`,
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
      const [firstTabId, ...remainingTabIds] = tabIds;

      if (firstTabId === undefined) continue;

      try {
        const groupId = await groupTabs([firstTabId, ...remainingTabIds], windowId);
        const color = groupColor(group.color);
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

  async function restoreWorkspace(input: unknown): Promise<RestoreReport> {
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

  return { captureWorkspace, restoreWorkspace };
}
