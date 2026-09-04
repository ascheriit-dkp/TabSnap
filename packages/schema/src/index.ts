import { z } from 'zod';

export const TABSNAP_FORMAT = 'tabsnap' as const;
export const TABSNAP_FORMAT_VERSION = 1 as const;

const MAX_WINDOWS = 1_000;
const MAX_TABS_PER_WINDOW = 10_000;
const MAX_GROUPS_PER_WINDOW = 1_000;
const MAX_URL_LENGTH = 2_097_152;
const MAX_TITLE_LENGTH = 10_000;
const MAX_COORDINATE = 1_000_000;

const orderSchema = z.number().int().nonnegative().max(1_000_000);

export const browserSchema = z.enum(['chrome', 'edge', 'firefox']);
export const platformSchema = z.enum(['windows', 'macos', 'linux']);
export const windowStateSchema = z.enum(['normal', 'maximized', 'minimized', 'fullscreen']);

export const boundsSchema = z
  .object({
    left: z.number().int().min(-MAX_COORDINATE).max(MAX_COORDINATE),
    top: z.number().int().min(-MAX_COORDINATE).max(MAX_COORDINATE),
    width: z.number().int().positive().max(MAX_COORDINATE),
    height: z.number().int().positive().max(MAX_COORDINATE),
  })
  .strict();

export const tabGroupSchema = z
  .object({
    id: z.string().min(1).max(256),
    order: orderSchema,
    title: z.string().max(MAX_TITLE_LENGTH).optional(),
    color: z.string().min(1).max(64).optional(),
    collapsed: z.boolean(),
  })
  .strict();

export const tabSchema = z
  .object({
    order: orderSchema,
    url: z.string().min(1).max(MAX_URL_LENGTH),
    title: z.string().max(MAX_TITLE_LENGTH).optional(),
    pinned: z.boolean(),
    active: z.boolean(),
    groupId: z.string().min(1).max(256).optional(),
  })
  .strict();

function hasDuplicates<T>(values: T[]): boolean {
  return new Set(values).size !== values.length;
}

export const windowSchema = z
  .object({
    order: orderSchema,
    state: windowStateSchema,
    focused: z.boolean(),
    bounds: boundsSchema.optional(),
    groups: z.array(tabGroupSchema).max(MAX_GROUPS_PER_WINDOW),
    tabs: z.array(tabSchema).min(1).max(MAX_TABS_PER_WINDOW),
  })
  .strict()
  .superRefine((window, ctx) => {
    if (hasDuplicates(window.tabs.map((tab) => tab.order))) {
      ctx.addIssue({
        code: 'custom',
        path: ['tabs'],
        message: 'Tab orders must be unique within a window.',
      });
    }

    if (window.tabs.filter((tab) => tab.active).length !== 1) {
      ctx.addIssue({
        code: 'custom',
        path: ['tabs'],
        message: 'A window must contain exactly one active tab.',
      });
    }

    if (hasDuplicates(window.groups.map((group) => group.id))) {
      ctx.addIssue({
        code: 'custom',
        path: ['groups'],
        message: 'Group IDs must be unique within a window.',
      });
    }

    if (hasDuplicates(window.groups.map((group) => group.order))) {
      ctx.addIssue({
        code: 'custom',
        path: ['groups'],
        message: 'Group orders must be unique within a window.',
      });
    }

    const groupIds = new Set(window.groups.map((group) => group.id));
    window.tabs.forEach((tab, index) => {
      if (tab.groupId !== undefined && !groupIds.has(tab.groupId)) {
        ctx.addIssue({
          code: 'custom',
          path: ['tabs', index, 'groupId'],
          message: 'Tab references an unknown group.',
        });
      }
    });
  });

export const sourceSchema = z
  .object({
    browser: browserSchema,
    browserVersion: z.string().min(1).max(256).optional(),
    platform: platformSchema,
  })
  .strict();

export const tabSnapSnapshotSchema = z
  .object({
    format: z.literal(TABSNAP_FORMAT),
    formatVersion: z.literal(TABSNAP_FORMAT_VERSION),
    createdAt: z.string().datetime({ offset: true }),
    source: sourceSchema,
    windows: z.array(windowSchema).min(1).max(MAX_WINDOWS),
  })
  .strict()
  .superRefine((snapshot, ctx) => {
    if (hasDuplicates(snapshot.windows.map((window) => window.order))) {
      ctx.addIssue({
        code: 'custom',
        path: ['windows'],
        message: 'Window orders must be unique.',
      });
    }

    if (snapshot.windows.filter((window) => window.focused).length > 1) {
      ctx.addIssue({
        code: 'custom',
        path: ['windows'],
        message: 'At most one window can be focused.',
      });
    }
  });

export type Browser = z.infer<typeof browserSchema>;
export type Platform = z.infer<typeof platformSchema>;
export type WindowState = z.infer<typeof windowStateSchema>;
export type Bounds = z.infer<typeof boundsSchema>;
export type TabGroup = z.infer<typeof tabGroupSchema>;
export type Tab = z.infer<typeof tabSchema>;
export type WindowSnapshot = z.infer<typeof windowSchema>;
export type SnapshotSource = z.infer<typeof sourceSchema>;
export type TabSnapSnapshot = z.infer<typeof tabSnapSnapshotSchema>;

export function parseTabSnapSnapshot(input: unknown): TabSnapSnapshot {
  return tabSnapSnapshotSchema.parse(input);
}

export function safeParseTabSnapSnapshot(input: unknown) {
  return tabSnapSnapshotSchema.safeParse(input);
}
