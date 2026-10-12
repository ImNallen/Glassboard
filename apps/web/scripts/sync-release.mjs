import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { createRelease, platforms } from '../src/lib/release.mjs';

export function manifestFromRelease(published) {
  assert(published && typeof published === 'object', 'GitHub returned invalid release metadata.');
  const tag = published.tag_name;
  assert.equal(published.draft, false, `Release ${tag} is still a draft.`);
  assert.equal(published.prerelease, false, `Release ${tag} is a prerelease.`);
  const version = typeof tag === 'string' ? /^v(.+)$/.exec(tag)?.[1] : undefined;
  assert(version, `Release tag ${tag} must look like v0.1.0.`);
  assert(Array.isArray(published.assets), 'GitHub returned no release assets.');

  const names = published.assets.map(asset => asset?.name).filter(name => typeof name === 'string');
  const assets = Object.fromEntries(Object.entries(platforms).map(([platform, { label, format }]) => {
    const matches = names.filter(name => name.endsWith(format));
    assert.equal(matches.length, 1, `Release ${tag} must have exactly one ${label} ${format} installer.`);
    return [platform, matches[0]];
  }));
  const manifest = { version, assets };
  createRelease(manifest);
  return manifest;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const token = process.env.GITHUB_TOKEN;
    const response = await fetch('https://api.github.com/repos/ImNallen/Glassboard/releases/latest', {
      headers: { Accept: 'application/vnd.github+json', ...(token && { Authorization: `Bearer ${token}` }) },
      signal: AbortSignal.timeout(20_000),
    });
    assert(response.ok, `GitHub returned ${response.status} for the latest release.`);
    const manifest = manifestFromRelease(await response.json());
    await writeFile(new URL('../src/data/release.json', import.meta.url), `${JSON.stringify(manifest, null, 2)}\n`);
    console.info(`release.json now advertises Glassboard ${manifest.version}.`);
  } catch (cause) {
    console.error(cause instanceof Error ? cause.message : cause);
    process.exitCode = 1;
  }
}
