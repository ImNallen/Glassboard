import { readFile, readdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const requiredPlatforms = ['darwin-aarch64', 'darwin-x86_64', 'windows-x86_64', 'linux-x86_64'];

export function rewriteManifest(manifest, { repository, tag, version, assets, signatures }) {
  if (manifest.version !== version) {
    throw new Error(`latest.json is for version ${manifest.version}, expected ${version}`);
  }
  for (const key of requiredPlatforms) {
    if (!manifest.platforms?.[key]) throw new Error(`latest.json is missing required platform ${key}`);
  }
  const downloadUrl = name => `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(name)}`;
  const platforms = Object.fromEntries(Object.entries(manifest.platforms).map(([key, entry]) => {
    const asset = assets.find(candidate => candidate.apiUrl === entry.url || downloadUrl(candidate.name) === entry.url);
    if (!asset) throw new Error(`${key} points to ${entry.url}, which is not an asset of ${tag}`);
    if (asset.state !== 'uploaded' || !(asset.size > 0)) {
      throw new Error(`${key} asset ${asset.name} is not fully uploaded (state ${asset.state}, size ${asset.size})`);
    }
    const expected = signatures[asset.name]?.trim();
    if (!expected) throw new Error(`${key} asset ${asset.name} has no ${asset.name}.sig to check its signature against`);
    if (entry.signature?.trim() !== expected) throw new Error(`${key} signature does not match ${asset.name}.sig`);
    return [key, { ...entry, url: downloadUrl(asset.name) }];
  }));
  return { ...manifest, platforms };
}

async function readJson(path) {
  try {
    return JSON.parse(await readFile(path, 'utf8'));
  } catch (error) {
    throw new Error(`${path}: ${error.message}`);
  }
}

async function main(args) {
  if (args.length !== 3) throw new Error('usage: node scripts/updater-manifest.mjs <dir> <tag> <version>');
  const [dir, tag, version] = args;
  const repository = process.env.GITHUB_REPOSITORY;
  if (!repository) throw new Error('GITHUB_REPOSITORY must name the repository, such as ImNallen/Glassboard');
  const manifest = await readJson(join(dir, 'latest.json'));
  const { assets } = await readJson(join(dir, 'assets.json'));
  if (!Array.isArray(assets)) throw new Error(`${join(dir, 'assets.json')} has no assets array`);
  const sigFiles = (await readdir(dir)).filter(name => name.endsWith('.sig'));
  const signatures = Object.fromEntries(await Promise.all(
    sigFiles.map(async name => [name.slice(0, -'.sig'.length), await readFile(join(dir, name), 'utf8')]),
  ));
  const rewritten = rewriteManifest(manifest, { repository, tag, version, assets, signatures });
  await writeFile(join(dir, 'latest.json'), `${JSON.stringify(rewritten, null, 2)}\n`);
  const urls = new Set(Object.values(rewritten.platforms).map(entry => entry.url));
  console.log(`latest.json for ${tag}: ${Object.keys(rewritten.platforms).length} platforms point at ${urls.size} release downloads.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    await main(process.argv.slice(2));
  } catch (cause) {
    console.error(cause instanceof Error ? cause.message : cause);
    process.exitCode = 1;
  }
}
