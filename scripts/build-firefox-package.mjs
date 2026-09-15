import { cp, copyFile, rm } from 'node:fs/promises';
import { resolve } from 'node:path';

const source = resolve('apps/chrome-extension/dist');
const target = resolve('apps/chrome-extension/dist-firefox');
const firefoxManifest = resolve('apps/chrome-extension/firefox-manifest.json');

await rm(target, { recursive: true, force: true });
await cp(source, target, { recursive: true });
await copyFile(firefoxManifest, resolve(target, 'manifest.json'));
