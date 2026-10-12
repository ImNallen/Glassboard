import assert from 'node:assert/strict';
import { test } from 'node:test';
import { manifestFromRelease } from './sync-release.mjs';

const latest = () => ({
  tag_name: 'v0.1.2',
  name: 'Glassboard v0.1.2',
  draft: false,
  prerelease: false,
  published_at: '2026-10-11T14:06:30Z',
  assets: [
    'Glassboard_0.1.2_amd64.AppImage',
    'Glassboard_0.1.2_amd64.AppImage.sig',
    'Glassboard_0.1.2_amd64.deb',
    'Glassboard_0.1.2_amd64.deb.sig',
    'Glassboard_0.1.2_universal.app.tar.gz',
    'Glassboard_0.1.2_universal.app.tar.gz.sig',
    'Glassboard_0.1.2_universal.dmg',
    'Glassboard_0.1.2_x64-setup.exe',
    'Glassboard_0.1.2_x64-setup.exe.sig',
    'latest.json',
  ].map(name => ({ name, state: 'uploaded', size: 100 })),
});

const without = suffix => {
  const release = latest();
  release.assets = release.assets.filter(asset => !asset.name.endsWith(suffix));
  return release;
};

test('advertises the installers of the latest published release and ignores updater artifacts', () => {
  assert.deepEqual(manifestFromRelease(latest()), {
    version: '0.1.2',
    assets: {
      macos: 'Glassboard_0.1.2_universal.dmg',
      windows: 'Glassboard_0.1.2_x64-setup.exe',
      linux: 'Glassboard_0.1.2_amd64.AppImage',
    },
  });
});

test('rejects drafts and prereleases', () => {
  assert.throws(() => manifestFromRelease({ ...latest(), draft: true }), /v0\.1\.2 is still a draft/);
  assert.throws(() => manifestFromRelease({ ...latest(), prerelease: true }), /v0\.1\.2 is a prerelease/);
});

test('rejects a release missing any platform installer', () => {
  assert.throws(() => manifestFromRelease(without('.dmg')), /exactly one macOS \.dmg installer/);
  assert.throws(() => manifestFromRelease(without('.exe')), /exactly one Windows \.exe installer/);
  assert.throws(() => manifestFromRelease(without('.AppImage')), /exactly one Linux .* \.AppImage installer/);
  assert.throws(() => manifestFromRelease({ ...latest(), assets: undefined }), /no release assets/);
});

test('rejects ambiguous installers instead of picking one', () => {
  const release = latest();
  release.assets.push({ name: 'Glassboard_0.1.2_aarch64.dmg', state: 'uploaded', size: 100 });
  assert.throws(() => manifestFromRelease(release), /exactly one macOS \.dmg installer/);
});

test('rejects tags that do not name a version', () => {
  for (const tag_name of ['0.1.2', 'vnext', undefined]) {
    assert.throws(() => manifestFromRelease({ ...latest(), tag_name }), /must look like v0\.1\.0|must name a version/);
  }
});
