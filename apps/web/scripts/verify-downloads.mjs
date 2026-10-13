import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { release } from '../src/lib/release.mjs';

export async function verifyPublicDownloads(fetchRequest = fetch, token = process.env.GITHUB_TOKEN) {
  const apiUrl = `https://api.github.com/repos/ImNallen/Glassboard/releases/tags/${release.tag}`;
  const options = () => ({ redirect: 'follow', signal: AbortSignal.timeout(20_000) });
  const response = await fetchRequest(apiUrl, {
    ...options(),
    // Shared runners exhaust the anonymous API limit. The installer requests below stay anonymous.
    headers: { Accept: 'application/vnd.github+json', ...(token && { Authorization: `Bearer ${token}` }) },
  });
  assert(response.ok, `Release ${release.tag} is not publicly available (${response.status}). Publish the complete draft before deploying.`);
  const published = await response.json();
  assert(published && typeof published === 'object', 'GitHub returned invalid release metadata.');
  assert.equal(published.tag_name, release.tag, 'GitHub returned a different release tag.');
  assert.equal(published.draft, false, 'The advertised release is still a draft.');
  assert(typeof published.published_at === 'string' && published.published_at.length > 0, 'The advertised release has not been published.');
  assert(Array.isArray(published.assets), 'GitHub returned no release assets.');

  for (const download of release.downloads) {
    const asset = published.assets.find(candidate => candidate && candidate.name === download.filename);
    assert(asset && asset.state === 'uploaded' && typeof asset.size === 'number' && asset.size > 0, `${download.label} installer ${download.filename} is missing, empty, or not uploaded.`);
    const file = await fetchRequest(download.url, { ...options(), method: 'HEAD' });
    assert(file.ok, `${download.label} download is not publicly available (${file.status}): ${download.url}`);
  }

  const manifestUrl = 'https://github.com/ImNallen/Glassboard/releases/latest/download/latest.json';
  const manifestResponse = await fetchRequest(manifestUrl, options());
  assert(manifestResponse.ok, `Updater manifest is not publicly available (${manifestResponse.status}): ${manifestUrl}`);
  const manifest = await manifestResponse.json();
  assert.equal(manifest?.version, release.version, 'The updater manifest names a different version.');
  const platforms = Object.entries(manifest.platforms ?? {});
  assert(platforms.length > 0, 'The updater manifest lists no platforms.');
  const downloadPrefix = `https://github.com/ImNallen/Glassboard/releases/download/${release.tag}/`;
  for (const [platform, entry] of platforms) {
    assert(typeof entry?.url === 'string' && entry.url.startsWith(downloadPrefix), `Updater manifest ${platform} URL is not a ${release.tag} release download: ${entry?.url}`);
  }
  for (const url of new Set(platforms.map(([, entry]) => entry.url))) {
    const file = await fetchRequest(url, { ...options(), method: 'HEAD' });
    assert(file.ok, `Updater download is not publicly available (${file.status}): ${url}`);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    await verifyPublicDownloads();
    console.info(`Public downloads verified for Glassboard ${release.version}.`);
  } catch (cause) {
    console.error(cause instanceof Error ? cause.message : cause);
    process.exitCode = 1;
  }
}
