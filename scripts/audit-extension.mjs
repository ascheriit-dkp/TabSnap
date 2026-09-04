import { readdir, readFile } from 'node:fs/promises';
import { extname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const dist = join(root, 'apps/chrome-extension/dist');
const manifestPath = join(dist, 'manifest.json');

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

const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
assert(manifest.manifest_version === 3, 'manifest_version must be 3.');

const permissions = [...(manifest.permissions ?? [])].sort();
const expectedPermissions = ['tabGroups', 'tabs'];
assert(
  JSON.stringify(permissions) === JSON.stringify(expectedPermissions),
  `permissions must be exactly ${expectedPermissions.join(', ')}; got ${permissions.join(', ') || 'none'}.`,
);

for (const key of [
  'host_permissions',
  'optional_host_permissions',
  'optional_permissions',
  'content_scripts',
  'externally_connectable',
]) {
  assert(!(key in manifest), `${key} must not be present.`);
}

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

const textExtensions = new Set(['.css', '.html', '.js', '.json']);
const networkPatterns = [
  { name: 'absolute HTTP(S) URL', pattern: /https?:\/\//u },
  { name: 'fetch()', pattern: /\bfetch\s*\(/u },
  { name: 'XMLHttpRequest', pattern: /\bXMLHttpRequest\b/u },
  { name: 'WebSocket', pattern: /\bWebSocket\b/u },
  { name: 'EventSource', pattern: /\bEventSource\b/u },
  { name: 'sendBeacon', pattern: /\bsendBeacon\b/u },
];

for (const file of await walk(dist)) {
  if (!textExtensions.has(extname(file)) || file.endsWith('.map')) continue;
  const contents = await readFile(file, 'utf8');
  const displayPath = relative(root, file);

  for (const { name, pattern } of networkPatterns) {
    assert(!pattern.test(contents), `${displayPath} contains ${name}.`);
  }
}

console.log('Extension audit passed: minimal permissions, no host access, no static network path.');
