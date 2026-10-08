import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRelease } from '../src/lib/release.mjs';

const metadata = () => ({
  version: '0.1.1',
  assets: {
    macos: 'Glassboard_0.1.1_universal.dmg',
    windows: 'Glassboard_0.1.1_x64-setup.exe',
  },
});

test('keeps the published 0.1.0 downloads without a Linux asset', () => {
  const previous = createRelease({
    version: '0.1.0',
    assets: { macos: 'Glassboard_0.1.0_universal.dmg', windows: 'Glassboard_0.1.0_x64-setup.exe' },
  });
  assert.equal(previous.version, '0.1.0');
  assert.deepEqual(previous.downloads.map(download => download.platform), ['macos', 'windows']);
  assert(previous.downloads.every(download => download.url.includes('/v0.1.0/')));
  assert.deepEqual(createRelease(metadata()).downloads.map(download => download.platform), ['macos', 'windows']);
});

test('adds an experimental X11 download only when an AppImage is named', () => {
  const next = metadata();
  next.assets.linux = 'Glassboard_0.1.1_amd64.AppImage';
  const linux = createRelease(next).downloads.find(download => download.platform === 'linux');
  assert.deepEqual(linux, {
    platform: 'linux',
    label: 'Linux (experimental X11)',
    architecture: '64-bit Intel and AMD',
    format: '.AppImage',
    filename: 'Glassboard_0.1.1_amd64.AppImage',
    url: 'https://github.com/ImNallen/Glassboard/releases/download/v0.1.1/Glassboard_0.1.1_amd64.AppImage',
  });
});

test('rejects invalid Linux installers and continues requiring macOS and Windows', () => {
  for (const filename of [undefined, null, '', 'Glassboard.deb', '../Glassboard.AppImage', 'Glass board.AppImage', 'Glassboard.appimage']) {
    const next = metadata();
    next.assets.linux = filename;
    assert.throws(() => createRelease(next), /invalid Linux/);
  }
  for (const platform of ['macos', 'windows']) {
    const next = metadata();
    delete next.assets[platform];
    next.assets.linux = 'Glassboard_0.1.1_amd64.AppImage';
    assert.throws(() => createRelease(next), /invalid .* installer filename/);
  }
  const next = metadata();
  next.assets.toString = 'extra.AppImage';
  assert.throws(() => createRelease(next), /unsupported platform/);
});
