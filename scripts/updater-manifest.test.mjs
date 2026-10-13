import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { rewriteManifest } from './updater-manifest.mjs';

const script = fileURLToPath(new URL('./updater-manifest.mjs', import.meta.url));
const api = id => `https://api.github.com/repos/ImNallen/Glassboard/releases/assets/${id}`;

const manifest = () => ({
  version: '0.1.3',
  notes: '',
  pub_date: '2026-10-12T08:25:12.953Z',
  platforms: {
    'linux-x86_64': { signature: 'c2lnLWFwcGltYWdl', url: api(633144694) },
    'linux-x86_64-appimage': { signature: 'c2lnLWFwcGltYWdl', url: api(633144694) },
    'linux-x86_64-deb': { signature: 'c2lnLWRlYg==', url: api(633144649) },
    'windows-x86_64': { signature: 'c2lnLWV4ZQ==', url: api(633146522) },
    'darwin-aarch64': { signature: 'c2lnLWFwcA==', url: api(633152398) },
    'darwin-x86_64': { signature: 'c2lnLWFwcA==', url: api(633152398) },
    'darwin-universal': { signature: 'c2lnLWFwcA==', url: api(633152398) },
  },
});

const assets = () => [
  { name: 'Glassboard_0.1.3_amd64.AppImage', state: 'uploaded', size: 85936632, apiUrl: api(633144694) },
  { name: 'Glassboard_0.1.3_amd64.AppImage.sig', state: 'uploaded', size: 440, apiUrl: api(633144811) },
  { name: 'Glassboard_0.1.3_amd64.deb', state: 'uploaded', size: 7657868, apiUrl: api(633144649) },
  { name: 'Glassboard_0.1.3_amd64.deb.sig', state: 'uploaded', size: 436, apiUrl: api(633144679) },
  { name: 'Glassboard_0.1.3_universal.app.tar.gz', state: 'uploaded', size: 11479231, apiUrl: api(633152398) },
  { name: 'Glassboard_0.1.3_universal.app.tar.gz.sig', state: 'uploaded', size: 428, apiUrl: api(633152439) },
  { name: 'Glassboard_0.1.3_x64-setup.exe', state: 'uploaded', size: 3508816, apiUrl: api(633146522) },
  { name: 'Glassboard_0.1.3_x64-setup.exe.sig', state: 'uploaded', size: 440, apiUrl: api(633146553) },
  { name: 'latest.json', state: 'uploaded', size: 6476, apiUrl: api(633152512) },
];

const signatures = () => ({
  'Glassboard_0.1.3_amd64.AppImage': 'c2lnLWFwcGltYWdl',
  'Glassboard_0.1.3_amd64.deb': 'c2lnLWRlYg==\n',
  'Glassboard_0.1.3_universal.app.tar.gz': 'c2lnLWFwcA==',
  'Glassboard_0.1.3_x64-setup.exe': 'c2lnLWV4ZQ==',
});

const expected = {
  version: '0.1.3',
  notes: '',
  pub_date: '2026-10-12T08:25:12.953Z',
  platforms: {
    'linux-x86_64': { signature: 'c2lnLWFwcGltYWdl', url: 'https://github.com/ImNallen/Glassboard/releases/download/v0.1.3/Glassboard_0.1.3_amd64.AppImage' },
    'linux-x86_64-appimage': { signature: 'c2lnLWFwcGltYWdl', url: 'https://github.com/ImNallen/Glassboard/releases/download/v0.1.3/Glassboard_0.1.3_amd64.AppImage' },
    'linux-x86_64-deb': { signature: 'c2lnLWRlYg==', url: 'https://github.com/ImNallen/Glassboard/releases/download/v0.1.3/Glassboard_0.1.3_amd64.deb' },
    'windows-x86_64': { signature: 'c2lnLWV4ZQ==', url: 'https://github.com/ImNallen/Glassboard/releases/download/v0.1.3/Glassboard_0.1.3_x64-setup.exe' },
    'darwin-aarch64': { signature: 'c2lnLWFwcA==', url: 'https://github.com/ImNallen/Glassboard/releases/download/v0.1.3/Glassboard_0.1.3_universal.app.tar.gz' },
    'darwin-x86_64': { signature: 'c2lnLWFwcA==', url: 'https://github.com/ImNallen/Glassboard/releases/download/v0.1.3/Glassboard_0.1.3_universal.app.tar.gz' },
    'darwin-universal': { signature: 'c2lnLWFwcA==', url: 'https://github.com/ImNallen/Glassboard/releases/download/v0.1.3/Glassboard_0.1.3_universal.app.tar.gz' },
  },
};

function rewrite(input = manifest(), overrides = {}) {
  return rewriteManifest(input, {
    repository: 'ImNallen/Glassboard', tag: 'v0.1.3', version: '0.1.3', assets: assets(), signatures: signatures(), ...overrides,
  });
}

test('rewrites API asset URLs to release download URLs and keeps everything else', () => {
  const result = rewrite();
  assert.deepEqual(result, expected);
  assert.deepEqual(Object.keys(result), ['version', 'notes', 'pub_date', 'platforms']);
  assert.deepEqual(Object.keys(result.platforms), Object.keys(expected.platforms));
});

test('rewriting an already rewritten manifest changes nothing', () => {
  const once = rewrite();
  assert.deepEqual(rewrite(once), once);
});

test('rejects a platform whose asset id was deleted by a re-upload', () => {
  const reuploaded = assets().map(asset => asset.name === 'Glassboard_0.1.3_amd64.deb' ? { ...asset, apiUrl: api(640000001) } : asset);
  assert.throws(() => rewrite(manifest(), { assets: reuploaded }), {
    message: `linux-x86_64-deb points to ${api(633144649)}, which is not an asset of v0.1.3`,
  });
});

test('rejects a stale or missing signature', () => {
  assert.throws(() => rewrite(manifest(), { signatures: { ...signatures(), 'Glassboard_0.1.3_x64-setup.exe': 'c2lnLW5ldw==' } }), {
    message: 'windows-x86_64 signature does not match Glassboard_0.1.3_x64-setup.exe.sig',
  });
  const { 'Glassboard_0.1.3_x64-setup.exe': _, ...withoutExe } = signatures();
  assert.throws(() => rewrite(manifest(), { signatures: withoutExe }), {
    message: 'windows-x86_64 asset Glassboard_0.1.3_x64-setup.exe has no Glassboard_0.1.3_x64-setup.exe.sig to check its signature against',
  });
});

test('rejects a manifest that lacks a required platform', () => {
  const input = manifest();
  delete input.platforms['darwin-x86_64'];
  assert.throws(() => rewrite(input), { message: 'latest.json is missing required platform darwin-x86_64' });
});

test('rejects a manifest for another version', () => {
  assert.throws(() => rewrite(manifest(), { version: '0.1.4' }), { message: 'latest.json is for version 0.1.3, expected 0.1.4' });
});

test('rejects an asset that has not finished uploading', () => {
  const uploading = assets().map(asset => asset.name === 'Glassboard_0.1.3_universal.app.tar.gz' ? { ...asset, state: 'starter' } : asset);
  assert.throws(() => rewrite(manifest(), { assets: uploading }), {
    message: 'darwin-aarch64 asset Glassboard_0.1.3_universal.app.tar.gz is not fully uploaded (state starter, size 11479231)',
  });
});

test('CLI rewrites latest.json in place from the downloaded release files', async t => {
  const dir = await mkdtemp(join(tmpdir(), 'glassboard-manifest-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  await writeFile(join(dir, 'latest.json'), JSON.stringify(manifest(), null, 2));
  await writeFile(join(dir, 'assets.json'), JSON.stringify({ assets: assets() }));
  for (const [name, text] of Object.entries(signatures())) await writeFile(join(dir, `${name}.sig`), text);
  const run = env => spawnSync(process.execPath, [script, dir, 'v0.1.3', '0.1.3'], { encoding: 'utf8', env: { ...process.env, ...env } });

  const missingRepository = run({ GITHUB_REPOSITORY: '' });
  assert.equal(missingRepository.status, 1);
  assert.equal(missingRepository.stderr, 'GITHUB_REPOSITORY must name the repository, such as ImNallen/Glassboard\n');

  const result = run({ GITHUB_REPOSITORY: 'ImNallen/Glassboard' });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, 'latest.json for v0.1.3: 7 platforms point at 4 release downloads.\n');
  assert.equal(await readFile(join(dir, 'latest.json'), 'utf8'), `${JSON.stringify(expected, null, 2)}\n`);
});
