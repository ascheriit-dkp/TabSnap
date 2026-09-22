import assert from 'node:assert/strict';
import { lstat, readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

function fail(message) {
  throw new Error('Release extension package verification failed: ' + message);
}

async function assertRegularTree(root, current = root) {
  for (const entry of await readdir(current, { withFileTypes: true })) {
    const full = path.join(current, entry.name);
    const info = await lstat(full);
    if (info.isSymbolicLink()) {
      fail('symbolic link in package: ' + path.relative(root, full));
    }
    if (info.isDirectory()) {
      await assertRegularTree(root, full);
    } else if (!info.isFile()) {
      fail('non-regular package entry: ' + path.relative(root, full));
    }
  }
}

async function requireFile(root, relativePath) {
  const full = path.join(root, relativePath);
  const info = await lstat(full).catch(() => null);
  if (!info?.isFile()) fail('missing package file: ' + relativePath);
}

const [tag, browser, directory] = process.argv.slice(2);
if (!tag || !browser || !directory) {
  fail(
    'usage: node scripts/verify-release-extension-package.mjs <tag> <chrome|edge|firefox> <extracted-directory>',
  );
}
if (!['chrome', 'edge', 'firefox'].includes(browser)) {
  fail('unsupported browser argument: ' + browser);
}

const expectedManifestPath =
  browser === 'firefox'
    ? 'apps/chrome-extension/firefox-manifest.json'
    : 'apps/chrome-extension/public/manifest.json';
const expectedManifest = JSON.parse(await readFile(expectedManifestPath, 'utf8'));
const manifest = JSON.parse(await readFile(path.join(directory, 'manifest.json'), 'utf8'));

try {
  assert.deepStrictEqual(manifest, expectedManifest);
} catch {
  fail(browser + ' release manifest differs from ' + expectedManifestPath);
}

if (manifest.version_name !== tag.slice(1)) {
  fail(browser + ' version_name ' + manifest.version_name + ' does not match ' + tag);
}

await assertRegularTree(directory);
await requireFile(directory, 'manifest.json');
await requireFile(directory, 'index.html');

const backgroundFiles = [
  manifest.background?.service_worker,
  ...(manifest.background?.scripts ?? []),
].filter(Boolean);
for (const file of backgroundFiles) await requireFile(directory, file);

for (const file of Object.values(manifest.icons ?? {})) {
  await requireFile(directory, file);
}
for (const file of Object.values(manifest.action?.default_icon ?? {})) {
  await requireFile(directory, file);
}

const index = await readFile(path.join(directory, 'index.html'), 'utf8');
for (const match of index.matchAll(/(?:src|href)="([^"]+)"/gu)) {
  const reference = match[1];
  if (
    reference.startsWith('http:') ||
    reference.startsWith('https:') ||
    reference.startsWith('data:') ||
    reference.startsWith('#')
  ) {
    continue;
  }
  const relative = reference.replace(/^\.\//u, '').replace(/^\//u, '');
  await requireFile(directory, relative);
}

process.stdout.write(browser + ' release package verified for ' + tag + '.\n');
