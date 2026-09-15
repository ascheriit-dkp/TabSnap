import { readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath, URL } from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const dist = join(root, 'apps/chrome-extension/dist');
const manifestPath = join(dist, 'manifest.json');
const companionOriginPermission = 'http://127.0.0.1/*';

function fail(message) {
  throw new Error(`Edge package audit failed: ${message}`);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));

assert(manifest.manifest_version === 3, 'manifest_version must be 3.');
assert(typeof manifest.name === 'string' && !/chrome/iu.test(manifest.name), 'name must not be Chrome-branded.');
assert(
  typeof manifest.description === 'string' && !/chrome/iu.test(manifest.description),
  'description must not be Chrome-branded.',
);
assert(!('update_url' in manifest), 'update_url must not be present in the Edge package.');
assert(!('minimum_edge_version' in manifest), 'minimum_edge_version is not a Chromium Edge manifest key.');
assert(
  typeof manifest.minimum_chrome_version === 'string' && manifest.minimum_chrome_version.length > 0,
  'minimum_chrome_version must be present for the shared Chromium package.',
);

const permissions = [...(manifest.permissions ?? [])].sort();
assert(
  JSON.stringify(permissions) === JSON.stringify(['tabGroups', 'tabs']),
  `permissions must be exactly tabGroups,tabs; got ${permissions.join(',') || 'none'}.`,
);

const optionalHostPermissions = [...(manifest.optional_host_permissions ?? [])].sort();
assert(
  JSON.stringify(optionalHostPermissions) === JSON.stringify([companionOriginPermission]),
  `optional_host_permissions must be exactly ${companionOriginPermission}.`,
);

for (const key of ['host_permissions', 'content_scripts', 'externally_connectable']) {
  assert(!(key in manifest), `${key} must not be present.`);
}

assert(
  manifest.background?.service_worker === 'background.js',
  'background.service_worker must be background.js.',
);
assert(manifest.action?.default_popup === undefined, 'the Edge toolbar action must open the persistent page.');

process.stdout.write(
  'Edge package audit passed: shared Chromium MV3 manifest, no Chrome branding/update URL, minimal permissions, and opt-in loopback companion access only.\n',
);
