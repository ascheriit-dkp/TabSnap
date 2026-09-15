import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import process from 'node:process';

const manifestPath = resolve('apps/chrome-extension/dist-firefox/manifest.json');
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
const expectedGeckoId = 'tabsnap@ascheriit-dkp.github.io';
const companionOriginPermission = 'http://127.0.0.1/*';

function assert(condition, message) {
  if (!condition) throw new Error(`Firefox package audit failed: ${message}`);
}

function sameStrings(actual, expected) {
  return (
    Array.isArray(actual) &&
    actual.length === expected.length &&
    [...actual].sort().every((value, index) => value === [...expected].sort()[index])
  );
}

assert(manifest.manifest_version === 3, 'manifest_version must be 3.');
assert(manifest.minimum_chrome_version === undefined, 'minimum_chrome_version must be absent.');
assert(
  sameStrings(manifest.permissions, ['tabs', 'tabGroups']),
  'permissions must stay exactly tabs + tabGroups.',
);
assert(
  sameStrings(manifest.optional_host_permissions, [companionOriginPermission]),
  'loopback companion access must remain optional.',
);

for (const key of [
  'host_permissions',
  'optional_permissions',
  'content_scripts',
  'externally_connectable',
  'update_url',
]) {
  assert(!(key in manifest), `${key} must not be present.`);
}

assert(manifest.background?.service_worker === undefined, 'background.service_worker must be absent.');
assert(
  sameStrings(manifest.background?.scripts, ['background.js']),
  'background.scripts must contain only background.js.',
);

const gecko = manifest.browser_specific_settings?.gecko;
assert(gecko?.id === expectedGeckoId, `Gecko ID must stay ${expectedGeckoId}.`);
assert(gecko?.strict_min_version === '139.0', 'Firefox minimum version must stay 139.0.');
assert(
  sameStrings(gecko?.data_collection_permissions?.required, ['none']),
  'AMO data collection declaration must remain required: none.',
);

const csp = manifest.content_security_policy?.extension_pages;
assert(typeof csp === 'string', 'extension_pages CSP is required.');
assert(
  csp.includes("script-src 'self' 'wasm-unsafe-eval'"),
  'CSP must keep scripts local while allowing packaged WASM.',
);
assert(!/https?:|\bconnect-src\b/u.test(csp), 'CSP must not enable remote sources.');

process.stdout.write('Firefox package audit passed.\n');
