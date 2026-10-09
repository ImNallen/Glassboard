import assert from 'node:assert/strict';
import { test } from 'node:test';
import { detectDesktopPlatform } from '../src/lib/platform-downloads.mjs';

test('recommends each desktop from low-entropy platform hints', () => {
  for (const [platform, expected] of [['macOS', 'macos'], ['Windows', 'windows'], ['Linux', 'linux']]) {
    assert.equal(detectDesktopPlatform({ userAgentData: { platform, mobile: false } }), expected);
  }
  assert.equal(detectDesktopPlatform({ userAgentData: { platform: 'Windows' }, platform: 'MacIntel' }), 'windows');
});

test('uses desktop browser identity when platform hints are absent or empty', () => {
  for (const [userAgent, platform, expected] of [
    ['Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)', 'MacIntel', 'macos'],
    ['Mozilla/5.0 (Windows NT 10.0; Win64; x64)', 'Win32', 'windows'],
    ['Mozilla/5.0 (X11; Linux x86_64)', 'Linux x86_64', 'linux'],
  ]) {
    assert.equal(detectDesktopPlatform({ userAgent, platform }), expected);
    assert.equal(detectDesktopPlatform({ userAgent, platform, userAgentData: { platform: '' } }), expected);
    assert.equal(detectDesktopPlatform({ platform }), expected);
  }
});

test('keeps all downloads for phones, tablets, and ChromeOS', () => {
  for (const userAgent of [
    'Mozilla/5.0 (Linux; Android 14) Mobile',
    'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)',
    'Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X)',
    'Mozilla/5.0 (iPod touch; CPU iPhone OS 15_0)',
    'Mozilla/5.0 (X11; CrOS x86_64 16093.0.0)',
    'Mozilla/5.0 (Linux) Mobile',
  ]) {
    assert.equal(detectDesktopPlatform({ userAgent }), null);
    assert.equal(detectDesktopPlatform({ userAgent, userAgentData: { platform: 'Linux' } }), null);
  }
  assert.equal(detectDesktopPlatform({ platform: 'MacIntel', maxTouchPoints: 5 }), null);
  assert.equal(detectDesktopPlatform({
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15)',
    maxTouchPoints: 5,
    userAgentData: { platform: 'macOS' },
  }), null);
  assert.equal(detectDesktopPlatform({ platform: 'MacIntel', maxTouchPoints: 0 }), 'macos');
  assert.equal(detectDesktopPlatform({ userAgentData: { platform: 'Windows', mobile: true } }), null);
});

test('does not contradict unsupported hints or guess unknown systems', () => {
  for (const platform of ['Android', 'iOS', 'Chrome OS', 'Unknown', 'FreeBSD']) {
    assert.equal(detectDesktopPlatform({ userAgentData: { platform }, userAgent: 'Windows NT 10.0', platform: 'Win32' }), null);
  }
  assert.equal(detectDesktopPlatform({ userAgent: 'Unknown browser', platform: 'FreeBSD' }), null);
  assert.equal(detectDesktopPlatform({}), null);
});
