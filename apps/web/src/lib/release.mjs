// @ts-check
import manifest from '../data/release.json' with { type: 'json' };

const repository = 'https://github.com/ImNallen/Glassboard';
const platforms = {
  macos: { label: 'macOS', architecture: 'Apple silicon and Intel', format: '.dmg' },
  windows: { label: 'Windows', architecture: '64-bit Intel and AMD', format: '.exe' },
  linux: { label: 'Linux (experimental X11)', architecture: '64-bit Intel and AMD', format: '.AppImage' },
};

/** @param {{ version: string, assets: { macos: string, windows: string, linux?: string } }} metadata */
export function createRelease(metadata) {
  if (!/^\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?$/.test(metadata.version)) {
    throw new Error('release.json must name a version such as 0.1.0.');
  }
  if (Object.keys(metadata.assets).some(platform => !Object.hasOwn(platforms, platform))) {
    throw new Error('release.json has an unsupported platform.');
  }
  /** @type {('macos' | 'windows' | 'linux')[]} */
  const supported = ['macos', 'windows'];
  if (Object.hasOwn(metadata.assets, 'linux')) supported.push('linux');
  const tag = `v${metadata.version}`;
  const downloads = supported.map(platform => {
    const filename = metadata.assets[platform];
    const details = platforms[platform];
    if (typeof filename !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(filename) || !filename.endsWith(details.format)) {
      throw new Error(`release.json has an invalid ${details.label} installer filename.`);
    }
    return { platform, ...details, filename, url: `${repository}/releases/download/${tag}/${filename}` };
  });
  return { version: metadata.version, tag, notesUrl: `${repository}/releases/tag/${tag}`, downloads };
}

export const release = createRelease(manifest);
