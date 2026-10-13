import assert from 'node:assert/strict';
import { test } from 'node:test';
import { release } from '../src/lib/release.mjs';
import { verifyPublicDownloads } from './verify-downloads.mjs';

const metadata = () => ({
  tag_name: release.tag,
  draft: false,
  published_at: '2026-10-03T12:00:00Z',
  assets: release.downloads.map(download => ({ name: download.filename, state: 'uploaded', size: 100 })),
});

const manifestUrl = 'https://github.com/ImNallen/Glassboard/releases/latest/download/latest.json';
const updaterDownload = name => `https://github.com/ImNallen/Glassboard/releases/download/${release.tag}/${name}`;
const updaterManifest = () => ({
  version: release.version,
  platforms: {
    'darwin-aarch64': { signature: 'c2ln', url: updaterDownload(`Glassboard_${release.version}_universal.app.tar.gz`) },
    'darwin-x86_64': { signature: 'c2ln', url: updaterDownload(`Glassboard_${release.version}_universal.app.tar.gz`) },
    'windows-x86_64': { signature: 'c2ln', url: updaterDownload(`Glassboard_${release.version}_x64-setup.exe`) },
    'linux-x86_64': { signature: 'c2ln', url: updaterDownload(`Glassboard_${release.version}_amd64.AppImage`) },
  },
});

function respond(url, options, manifest = updaterManifest()) {
  if (options.method === 'HEAD') return new Response(null);
  return Response.json(url === manifestUrl ? manifest : metadata());
}

test('verifies every exact public download URL without authorization and follows redirects', async () => {
  const requests = [];
  await verifyPublicDownloads(async (url, options) => {
    requests.push({ url, options });
    return respond(url, options);
  }, '');
  const updaterUrls = [
    updaterDownload(`Glassboard_${release.version}_universal.app.tar.gz`),
    updaterDownload(`Glassboard_${release.version}_x64-setup.exe`),
    updaterDownload(`Glassboard_${release.version}_amd64.AppImage`),
  ];
  assert.deepEqual(requests.map(request => request.url), [
    `https://api.github.com/repos/ImNallen/Glassboard/releases/tags/${release.tag}`,
    ...release.downloads.map(download => download.url),
    manifestUrl,
    ...updaterUrls,
  ]);
  for (const { options } of requests) {
    assert.equal(new Headers(options.headers).has('authorization'), false);
    assert.equal(options.redirect, 'follow');
    assert(options.signal instanceof AbortSignal);
  }
  assert.deepEqual(requests.map(request => request.options.method), [
    undefined, ...release.downloads.map(() => 'HEAD'), undefined, ...updaterUrls.map(() => 'HEAD'),
  ]);
});

test('authorizes only the release metadata request when a token is set', async () => {
  const authorization = [];
  await verifyPublicDownloads(async (url, options) => {
    authorization.push(new Headers(options.headers).get('authorization'));
    return respond(url, options);
  }, 'secret');
  assert.deepEqual(authorization, ['Bearer secret', ...release.downloads.map(() => null), null, null, null, null]);
});

test('rejects an unpublished draft before checking installer URLs', async () => {
  let requests = 0;
  await assert.rejects(verifyPublicDownloads(async () => {
    requests++;
    return Response.json({ ...metadata(), draft: true, published_at: null });
  }), /still a draft/);
  assert.equal(requests, 1);
});

test('rejects a release that lacks the Windows installer', async () => {
  await assert.rejects(verifyPublicDownloads(async (_url, options) => {
    return options.method === 'HEAD' ? new Response(null) : Response.json({ ...metadata(), assets: metadata().assets.slice(0, 1) });
  }), /Windows installer .* missing, empty, or not uploaded/);
});

test('rejects an unreachable Windows download even when its metadata exists', async () => {
  await assert.rejects(verifyPublicDownloads(async (url, options) => {
    if (options.method !== 'HEAD') return Response.json(metadata());
    return new Response(null, { status: url.endsWith('.exe') ? 404 : 200 });
  }), /Windows download is not publicly available \(404\)/);
});

test('rejects an anonymously inaccessible release', async () => {
  await assert.rejects(verifyPublicDownloads(async () => new Response(null, { status: 404 })), /Publish the complete draft/);
});

test('rejects a mismatched tag or an empty installer', async () => {
  await assert.rejects(verifyPublicDownloads(async () => Response.json({ ...metadata(), tag_name: 'v9.9.9' })), /different release tag/);
  const empty = metadata();
  empty.assets[0].size = 0;
  await assert.rejects(verifyPublicDownloads(async () => Response.json(empty)), /macOS installer .* missing, empty, or not uploaded/);
});


test('rejects an installer that has not finished uploading', async () => {
  const uploading = metadata();
  uploading.assets[0].state = 'starter';
  await assert.rejects(verifyPublicDownloads(async () => Response.json(uploading)), /not uploaded/);
});

test('rejects an updater manifest that still points at the rate-limited asset API', async () => {
  const manifest = updaterManifest();
  manifest.platforms['windows-x86_64'].url = 'https://api.github.com/repos/ImNallen/Glassboard/releases/assets/633146522';
  await assert.rejects(verifyPublicDownloads(async (url, options) => respond(url, options, manifest)),
    /Updater manifest windows-x86_64 URL is not a v[\d.]+ release download: https:\/\/api\.github\.com\//);
});

test('rejects an updater manifest for another version', async () => {
  await assert.rejects(verifyPublicDownloads(async (url, options) => respond(url, options, { ...updaterManifest(), version: '9.9.9' })),
    /updater manifest names a different version/);
});

test('rejects an unreachable updater download', async () => {
  await assert.rejects(verifyPublicDownloads(async (url, options) => {
    return url.endsWith('.app.tar.gz') ? new Response(null, { status: 404 }) : respond(url, options);
  }), /Updater download is not publicly available \(404\): .*universal\.app\.tar\.gz/);
});
