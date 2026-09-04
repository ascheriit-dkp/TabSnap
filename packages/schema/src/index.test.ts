import { describe, expect, it } from 'vitest';

import {
  TABSNAP_FORMAT,
  TABSNAP_FORMAT_VERSION,
  safeParseTabSnapSnapshot,
  type TabSnapSnapshot,
} from './index.js';

function validSnapshot(): TabSnapSnapshot {
  return {
    format: TABSNAP_FORMAT,
    formatVersion: TABSNAP_FORMAT_VERSION,
    createdAt: '2026-09-04T14:00:00+02:00',
    source: {
      browser: 'chrome',
      browserVersion: '140.0.0.0',
      platform: 'windows',
    },
    windows: [
      {
        order: 0,
        state: 'normal',
        focused: true,
        bounds: { left: 0, top: 0, width: 1920, height: 1080 },
        groups: [
          {
            id: 'research',
            order: 0,
            title: 'Research',
            color: 'blue',
            collapsed: false,
          },
        ],
        tabs: [
          {
            order: 0,
            url: 'https://github.com/ascheriit-dkp/TabSnap',
            title: 'TabSnap',
            pinned: true,
            active: false,
            groupId: 'research',
          },
          {
            order: 1,
            url: 'https://example.com/',
            pinned: false,
            active: true,
          },
        ],
      },
    ],
  };
}

describe('tabSnapSnapshotSchema', () => {
  it('accepts a valid snapshot', () => {
    expect(safeParseTabSnapSnapshot(validSnapshot()).success).toBe(true);
  });

  it('rejects unknown fields', () => {
    const snapshot = { ...validSnapshot(), surprise: true };
    expect(safeParseTabSnapSnapshot(snapshot).success).toBe(false);
  });

  it('rejects unsupported format versions', () => {
    const snapshot = { ...validSnapshot(), formatVersion: 2 };
    expect(safeParseTabSnapSnapshot(snapshot).success).toBe(false);
  });

  it('rejects duplicate window orders', () => {
    const snapshot = validSnapshot();
    snapshot.windows.push({ ...snapshot.windows[0]!, focused: false });
    expect(safeParseTabSnapSnapshot(snapshot).success).toBe(false);
  });

  it('rejects duplicate tab orders', () => {
    const snapshot = validSnapshot();
    snapshot.windows[0]!.tabs[1]!.order = 0;
    expect(safeParseTabSnapSnapshot(snapshot).success).toBe(false);
  });

  it('requires exactly one active tab per window', () => {
    const snapshot = validSnapshot();
    snapshot.windows[0]!.tabs[1]!.active = false;
    expect(safeParseTabSnapSnapshot(snapshot).success).toBe(false);
  });

  it('rejects unknown group references', () => {
    const snapshot = validSnapshot();
    snapshot.windows[0]!.tabs[0]!.groupId = 'missing';
    expect(safeParseTabSnapSnapshot(snapshot).success).toBe(false);
  });

  it('rejects more than one focused window', () => {
    const snapshot = validSnapshot();
    snapshot.windows.push({ ...snapshot.windows[0]!, order: 1 });
    expect(safeParseTabSnapSnapshot(snapshot).success).toBe(false);
  });
});
