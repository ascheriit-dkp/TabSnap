import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { readdir, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

function fail(message) {
  throw new Error('Release asset verification failed: ' + message);
}

async function sha256(filePath) {
  const hash = createHash('sha256');
  await new Promise((resolve, reject) => {
    const stream = createReadStream(filePath);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', resolve);
    stream.on('error', reject);
  });
  return hash.digest('hex');
}

const [tag, directory] = process.argv.slice(2);
if (!tag || !directory) {
  fail('usage: node scripts/verify-release-assets.mjs <vX.Y.Z...> <asset-directory>');
}
if (!/^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$/u.test(tag)) {
  fail('invalid release tag: ' + tag);
}

const version = tag.slice(1);
const rootPackage = JSON.parse(await readFile('package.json', 'utf8'));
if (rootPackage.version !== version) {
  fail('source version ' + rootPackage.version + ' does not match ' + tag);
}

const archiveNames = [
  'tabsnap-chrome-' + tag + '.zip',
  'tabsnap-edge-' + tag + '.zip',
  'tabsnap-firefox-' + tag + '.zip',
  'tabsnap-companion-windows-x64-' + tag + '.zip',
];
const expectedNames = archiveNames.flatMap((name) => [name, name + '.sha256']).sort();

const entries = await readdir(directory, { withFileTypes: true });
const actualNames = entries.map((entry) => entry.name).sort();
if (JSON.stringify(actualNames) !== JSON.stringify(expectedNames)) {
  fail(
    'expected exactly 8 release assets:\n' +
      expectedNames.join('\n') +
      '\nfound:\n' +
      actualNames.join('\n'),
  );
}

for (const entry of entries) {
  if (!entry.isFile()) fail('asset is not a regular file: ' + entry.name);
}

for (const archiveName of archiveNames) {
  const archivePath = path.join(directory, archiveName);
  const checksumPath = path.join(directory, archiveName + '.sha256');
  const archiveStat = await stat(archivePath);
  if (archiveStat.size === 0) fail('archive is empty: ' + archiveName);

  const checksumText = (await readFile(checksumPath, 'ascii')).trim();
  const match = /^([0-9a-f]{64}) {2}([^\r\n]+)$/u.exec(checksumText);
  if (!match) fail('invalid checksum format: ' + archiveName + '.sha256');
  if (match[2] !== archiveName) {
    fail('checksum names ' + match[2] + ' instead of ' + archiveName);
  }

  const actual = await sha256(archivePath);
  if (actual !== match[1]) {
    fail('SHA-256 mismatch for ' + archiveName + ': expected ' + match[1] + ', got ' + actual);
  }
}

process.stdout.write('Published release assets verified for ' + tag + '.\n');
