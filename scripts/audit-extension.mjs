import { readdir, readFile } from 'node:fs/promises';
import { extname, join, relative, resolve } from 'node:path';
import process from 'node:process';
import { fileURLToPath, URL } from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const extensionRoot = join(root, 'apps/chrome-extension');
const extensionSource = join(extensionRoot, 'src');
const dist = join(extensionRoot, 'dist');
const manifestPath = join(dist, 'manifest.json');
const companionOriginPermission = 'http://127.0.0.1/*';

function fail(message) {
  throw new Error(`Extension audit failed: ${message}`);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await walk(path)));
    else files.push(path);
  }

  return files;
}

function rejectMatch(contents, displayPath, name, pattern) {
  const match = pattern.exec(contents);
  if (match === null) return;

  const start = Math.max(0, match.index - 120);
  const end = Math.min(contents.length, match.index + match[0].length + 180);
  const context = contents.slice(start, end).replaceAll('\n', ' ');
  fail(`${displayPath} contains ${name} near ${JSON.stringify(context)}.`);
}

const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
assert(manifest.manifest_version === 3, 'manifest_version must be 3.');

const permissions = [...(manifest.permissions ?? [])].sort();
const expectedPermissions = ['tabGroups', 'tabs'];
assert(
  JSON.stringify(permissions) === JSON.stringify(expectedPermissions),
  `permissions must be exactly ${expectedPermissions.join(', ')}; got ${permissions.join(', ') || 'none'}.`,
);

const optionalHostPermissions = [...(manifest.optional_host_permissions ?? [])].sort();
assert(
  JSON.stringify(optionalHostPermissions) === JSON.stringify([companionOriginPermission]),
  `optional_host_permissions must be exactly ${companionOriginPermission}.`,
);

for (const key of [
  'host_permissions',
  'optional_permissions',
  'content_scripts',
  'externally_connectable',
]) {
  assert(!(key in manifest), `${key} must not be present.`);
}

assert(
  manifest.action?.default_popup === undefined,
  'the toolbar action must not use an ephemeral popup.',
);
assert(
  manifest.background?.service_worker === 'background.js',
  'background.service_worker must be background.js.',
);

const csp = manifest.content_security_policy?.extension_pages;
assert(typeof csp === 'string', 'extension_pages CSP is required.');
assert(
  csp.includes("script-src 'self' 'wasm-unsafe-eval'"),
  'CSP must allow only local scripts plus packaged WASM.',
);
assert(
  !/https?:|\bconnect-src\b|(?<!wasm-)unsafe-eval/u.test(csp),
  'CSP must not enable remote network/script sources.',
);

const forbiddenNetworkPatterns = [
  { name: 'XMLHttpRequest', pattern: /\bXMLHttpRequest\b/u },
  { name: 'WebSocket', pattern: /\bWebSocket\b/u },
  { name: 'EventSource', pattern: /\bEventSource\b/u },
  { name: 'sendBeacon', pattern: /\bsendBeacon\b/u },
  { name: 'importScripts()', pattern: /\bimportScripts\s*\(/u },
];

// Audit TabSnap's production source for actual destinations before dependencies are bundled in.
// Tests deliberately contain hostile/non-loopback examples and are excluded here.
for (const file of await walk(extensionSource)) {
  if (extname(file) !== '.ts' || file.endsWith('.test.ts')) continue;

  const contents = await readFile(file, 'utf8');
  const displayPath = relative(root, file);
  rejectMatch(
    contents,
    displayPath,
    'non-loopback absolute HTTP(S) URL',
    /https?:\/\/(?!127\.0\.0\.1(?::|\/))/u,
  );
}

const textExtensions = new Set(['.css', '.html', '.js', '.json']);
const remoteResourceExtensions = new Set(['.css', '.html', '.json']);

for (const file of await walk(dist)) {
  const extension = extname(file);
  if (!textExtensions.has(extension) || file.endsWith('.map')) continue;

  const contents = await readFile(file, 'utf8');
  const displayPath = relative(root, file);

  if (remoteResourceExtensions.has(extension) && file !== manifestPath) {
    rejectMatch(contents, displayPath, 'absolute HTTP(S) resource URL', /https?:\/\//u);
  }

  if (extension === '.js') {
    for (const { name, pattern } of forbiddenNetworkPatterns) {
      rejectMatch(contents, displayPath, name, pattern);
    }

    if (/\bfetch\b/u.test(contents)) {
      assert(
        contents.includes('http://127.0.0.1:'),
        `${displayPath} uses fetch without the fixed IPv4 loopback endpoint.`,
      );
    }
  }
}

process.stdout.write(
  'Extension audit passed: minimal browser permissions, opt-in IPv4 loopback access only, persistent page launcher, no remote resources, and no general-purpose network primitives.\n',
);
