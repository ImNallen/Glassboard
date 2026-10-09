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

test('verifies every exact public download URL without authorization and follows redirects', async () => {
  const requests = [];
  await verifyPublicDownloads(async (url, options) => {
    requests.push({ url, options });
    return options.method === 'HEAD' ? new Response(null) : Response.json(metadata());
  });
  assert.deepEqual(requests.map(request => request.url), [
    `https://api.github.com/repos/ImNallen/Glassboard/releases/tags/${release.tag}`,
    ...release.downloads.map(download => download.url),
  ]);
  for (const { options } of requests) {
    assert.equal(new Headers(options.headers).has('authorization'), false);
    assert.equal(options.redirect, 'follow');
    assert(options.signal instanceof AbortSignal);
  }
  assert.deepEqual(requests.slice(1).map(request => request.options.method), release.downloads.map(() => 'HEAD'));
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
