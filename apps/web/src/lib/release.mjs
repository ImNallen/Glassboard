// @ts-check
import manifest from '../data/release.json' with { type: 'json' };

const repository = 'https://github.com/ImNallen/Glassboard';
const platforms = {
  macos: { label: 'macOS', architecture: 'Apple silicon and Intel', format: '.dmg' },
  windows: { label: 'Windows', architecture: '64-bit Intel and AMD', format: '.exe' },
};
/** @type {('macos' | 'windows')[]} */
const supported = ['macos', 'windows'];

if (!/^\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?$/.test(manifest.version)) {
  throw new Error('release.json must name a version such as 0.1.0.');
}
if (Object.keys(manifest.assets).length !== supported.length) {
  throw new Error('release.json must name exactly one macOS installer and one Windows installer.');
}

const tag = `v${manifest.version}`;
const downloads = supported.map(platform => {
  const filename = manifest.assets[platform];
  const details = platforms[platform];
  if (typeof filename !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(filename) || !filename.endsWith(details.format)) {
    throw new Error(`release.json has an invalid ${details.label} installer filename.`);
  }
  return { platform, ...details, filename, url: `${repository}/releases/download/${tag}/${filename}` };
});

export const release = { version: manifest.version, tag, notesUrl: `${repository}/releases/tag/${tag}`, downloads };
