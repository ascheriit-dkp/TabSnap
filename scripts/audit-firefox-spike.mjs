import { readFile } from 'node:fs/promises';

const manifestPath = new URL('../tests/fixtures/firefox-manifest-v3.json', import.meta.url);
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function sameStrings(actual, expected) {
  return (
    Array.isArray(actual) &&
    actual.length === expected.length &&
    [...actual].sort().every((value, index) => value === [...expected].sort()[index])
  );
}

assert(manifest.manifest_version === 3, 'Firefox spike manifest must use MV3.');
assert(
  manifest.minimum_chrome_version === undefined,
  'Firefox manifest must not contain minimum_chrome_version.',
);
assert(
  sameStrings(manifest.permissions, ['tabs', 'tabGroups']),
  'Firefox permissions must stay tabs + tabGroups.',
);
assert(
  sameStrings(manifest.optional_host_permissions, ['http://127.0.0.1/*']),
  'Loopback companion access must remain optional.',
);

assert(
  manifest.background?.service_worker === undefined,
  'Firefox must not depend on background.service_worker.',
);
assert(
  sameStrings(manifest.background?.scripts, ['background.js']),
  'Firefox MV3 must use background.scripts with the existing background entrypoint.',
);

const gecko = manifest.browser_specific_settings?.gecko;
assert(
  typeof gecko?.id === 'string' && gecko.id.length > 0,
  'Firefox MV3 signing needs a Gecko extension ID.',
);
assert(
  gecko.strict_min_version === '139.0',
  'Firefox support baseline must remain 139.0 for full tabGroups support.',
);
assert(
  sameStrings(gecko.data_collection_permissions?.required, ['none']),
  'Firefox AMO data collection declaration must state required: none.',
);

assert(manifest.content_scripts === undefined, 'Firefox spike must not introduce content scripts.');
assert(
  manifest.host_permissions === undefined,
  'Firefox spike must not introduce required host permissions.',
);
assert(manifest.update_url === undefined, 'Firefox spike must not introduce an update URL.');

console.log('Firefox compatibility manifest audit passed.');
