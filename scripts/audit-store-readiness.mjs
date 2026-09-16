import { readFile } from 'node:fs/promises';
import process from 'node:process';

const chromiumManifest = JSON.parse(
  await readFile('apps/chrome-extension/public/manifest.json', 'utf8'),
);
const firefoxManifest = JSON.parse(
  await readFile('apps/chrome-extension/firefox-manifest.json', 'utf8'),
);
const privacy = await readFile('docs/privacy.md', 'utf8');
const submission = await readFile('docs/guide/store-submission.md', 'utf8');
const listing = await readFile('store/listing.md', 'utf8');
const reviewerNotes = await readFile('store/reviewer-notes.md', 'utf8');

const companionOrigin = 'http://127.0.0.1/*';

function assert(condition, message) {
  if (!condition) throw new Error(`Store readiness audit failed: ${message}`);
}

function sameStrings(actual, expected) {
  return (
    Array.isArray(actual) &&
    actual.length === expected.length &&
    [...actual].sort().every((value, index) => value === [...expected].sort()[index])
  );
}

for (const [name, document] of [
  ['privacy policy', privacy],
  ['submission guide', submission],
  ['store listing', listing],
  ['reviewer notes', reviewerNotes],
]) {
  assert(document.trim().length > 200, `${name} is missing or suspiciously short.`);
  assert(!/\b(?:TODO|TBD|PLACEHOLDER)\b/u.test(document), `${name} contains an unresolved placeholder.`);
}

for (const [browser, manifest] of [
  ['Chromium', chromiumManifest],
  ['Firefox', firefoxManifest],
]) {
  assert(manifest.manifest_version === 3, `${browser} package must use Manifest V3.`);
  assert(
    sameStrings(manifest.permissions, ['tabs', 'tabGroups']),
    `${browser} required permissions must stay exactly tabs + tabGroups.`,
  );
  assert(
    sameStrings(manifest.optional_host_permissions, [companionOrigin]),
    `${browser} loopback access must remain optional and IPv4-only.`,
  );
  assert(manifest.host_permissions === undefined, `${browser} must not add required host permissions.`);
  assert(manifest.content_scripts === undefined, `${browser} must not add content scripts.`);
  assert(manifest.update_url === undefined, `${browser} must not add a custom update URL.`);
}

const gecko = firefoxManifest.browser_specific_settings?.gecko;
assert(gecko?.id === 'tabsnap@ascheriit-dkp.github.io', 'Firefox Gecko ID changed.');
assert(gecko?.strict_min_version === '140.0', 'Firefox minimum must stay 140.0.');
assert(
  sameStrings(gecko?.data_collection_permissions?.required, ['none']),
  'Firefox extension-only mode must require no data collection.',
);
assert(
  sameStrings(gecko?.data_collection_permissions?.optional, ['browsingActivity']),
  'Firefox companion mode must declare optional browsingActivity.',
);

assert(
  privacy.includes('127.0.0.1') && privacy.includes('encrypted'),
  'Privacy policy must disclose encrypted loopback companion behavior.',
);
assert(
  listing.includes('No account.') && listing.includes('No analytics.'),
  'Store listing must preserve the local-first disclosure.',
);
assert(
  reviewerNotes.includes('pnpm extension:firefox-lint'),
  'AMO reviewer notes must include the reproducible Firefox validation command.',
);

process.stdout.write('Store readiness audit passed.\n');
