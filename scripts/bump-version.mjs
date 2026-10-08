import { readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const paths = {
  desktop: new URL('../apps/desktop/package.json', import.meta.url),
  npmLock: new URL('../package-lock.json', import.meta.url),
  cargo: new URL('../apps/desktop/src-tauri/Cargo.toml', import.meta.url),
  cargoLock: new URL('../apps/desktop/src-tauri/Cargo.lock', import.meta.url),
};
const tauriPath = new URL('../apps/desktop/src-tauri/tauri.conf.json', import.meta.url);
const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

function json(source, label) {
  try {
    return JSON.parse(source);
  } catch (error) {
    throw new Error(`${label}: ${error.message}`);
  }
}

function version(value, label) {
  const match = typeof value === 'string' && semver.exec(value);
  if (!match || match[4]?.split('.').some(identifier => /^\d+$/.test(identifier) && identifier.length > 1 && identifier.startsWith('0'))) {
    throw new Error(`${label} has an invalid version: ${String(value)}`);
  }
  return value;
}

function rustField(source, header, packageName) {
  const sections = [...source.matchAll(/^\[(\[?)[^\]\r\n]+\]\]?\s*$/gm)];
  const candidates = [];
  for (let i = 0; i < sections.length; i++) {
    if (sections[i][0].trim() !== header) continue;
    const start = sections[i].index + sections[i][0].length;
    const end = sections[i + 1]?.index ?? source.length;
    const body = source.slice(start, end);
    if (packageName && !/^name\s*=\s*"glassboard"\s*$/m.test(body)) continue;
    const fields = [...body.matchAll(/^version\s*=\s*"([^"]*)"\s*$/gm)];
    if (fields.length !== 1) throw new Error(`${header} must have one version field`);
    const field = fields[0];
    const quoteOffset = field[0].indexOf('"');
    candidates.push({ value: field[1], start: start + field.index + quoteOffset + 1 });
  }
  if (candidates.length !== 1) throw new Error(`expected one ${header}${packageName ? ' glassboard' : ''} section`);
  return candidates[0];
}

function inspect(originals, tauriSource) {
  const desktop = json(originals.desktop.toString('utf8'), 'desktop package.json');
  const npmLock = json(originals.npmLock.toString('utf8'), 'package-lock.json');
  const tauri = json(tauriSource, 'tauri.conf.json');
  if (desktop.name !== '@glassboard/desktop' || npmLock.packages?.['apps/desktop']?.name !== '@glassboard/desktop') {
    throw new Error('desktop package identity is missing from its manifest or npm lock');
  }
  if (tauri.version !== '../package.json') throw new Error('Tauri must read ../package.json for its version');

  const cargo = rustField(originals.cargo.toString('utf8'), '[package]', 'glassboard');
  const cargoLock = rustField(originals.cargoLock.toString('utf8'), '[[package]]', 'glassboard');
  const copies = {
    desktop: version(desktop.version, 'desktop package.json'),
    npmLock: version(npmLock.packages['apps/desktop'].version, 'package-lock.json desktop entry'),
    cargo: version(cargo.value, 'Cargo.toml package'),
    cargoLock: version(cargoLock.value, 'Cargo.lock glassboard package'),
  };
  if (new Set(Object.values(copies)).size !== 1) {
    throw new Error(`version mismatch: ${Object.entries(copies).map(([file, value]) => `${file}=${value}`).join(', ')}`);
  }
  return { currentVersion: copies.desktop, originals, rustFields: { cargo, cargoLock } };
}

function changeRustVersion(source, field, nextVersion) {
  return source.slice(0, field.start) + nextVersion + source.slice(field.start + field.value.length);
}

async function restore(originals) {
  await Promise.all(Object.entries(paths).map(async ([key, path]) => writeFile(path, originals[key])));
}

async function main() {
  const args = process.argv.slice(2);
  const check = args.length === 1 && args[0] === '--check';
  if (!check && (args.length !== 1 || (!['patch', 'minor', 'major'].includes(args[0]) && !semver.test(args[0])))) {
    throw new Error('usage: npm run version:bump -- patch|minor|major|<semver> (or npm run version:check)');
  }
  const explicitVersion = !check && !['patch', 'minor', 'major'].includes(args[0]) ? version(args[0], 'requested version') : undefined;
  if (explicitVersion?.includes('+')) {
    throw new Error('requested version has build metadata, which npm version drops; use a version without +metadata');
  }
  const originals = Object.fromEntries(await Promise.all(Object.entries(paths).map(async ([key, path]) => [key, await readFile(path)])));
  const plan = inspect(originals, await readFile(tauriPath, 'utf8'));
  if (check) {
    console.log(`Version copies agree: ${plan.currentVersion}`);
    return;
  }

  if (!process.env.npm_execpath) throw new Error('run version:bump through npm so its CLI path is available');
  try {
    const result = spawnSync(process.execPath, [
      process.env.npm_execpath, 'version', args[0], '--workspace=apps/desktop',
      '--include-workspace-root=false', '--no-git-tag-version', '--ignore-scripts',
      '--package-lock-only', '--package-lock=true', '--workspaces-update=true',
      '--offline', '--allow-same-version',
    ], { cwd: root, encoding: 'utf8' });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`npm version failed: ${(result.stderr || result.stdout || `exit ${result.status}`).trim()}`);

    const updatedDesktop = json(await readFile(paths.desktop, 'utf8'), 'updated desktop package.json');
    const nextVersion = version(updatedDesktop.version, 'updated desktop package.json');
    if (explicitVersion && nextVersion !== explicitVersion) {
      throw new Error(`npm returned ${nextVersion} instead of requested version ${explicitVersion}`);
    }
    const updatedLock = json(await readFile(paths.npmLock, 'utf8'), 'updated package-lock.json');
    if (updatedLock.packages?.['apps/desktop']?.version !== nextVersion) {
      throw new Error('npm did not update the desktop package-lock entry');
    }
    for (const key of ['cargo', 'cargoLock']) {
      await writeFile(paths[key], changeRustVersion(originals[key].toString('utf8'), plan.rustFields[key], nextVersion));
    }
    const finalFiles = Object.fromEntries(await Promise.all(Object.entries(paths).map(async ([key, path]) => [key, await readFile(path)])));
    const finalPlan = inspect(finalFiles, await readFile(tauriPath, 'utf8'));
    if (finalPlan.currentVersion !== nextVersion) throw new Error('final version differs from npm result');
    console.log(`Version bumped: ${plan.currentVersion} -> ${nextVersion}`);
  } catch (error) {
    await restore(originals);
    throw error;
  }
}

main().catch(error => {
  console.error(error.message);
  process.exitCode = 1;
});
