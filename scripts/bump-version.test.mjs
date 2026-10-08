import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, mkdir, readFile, writeFile, copyFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const script = fileURLToPath(new URL('./bump-version.mjs', import.meta.url));
const files = {
  desktop: 'apps/desktop/package.json',
  npmLock: 'package-lock.json',
  cargo: 'apps/desktop/src-tauri/Cargo.toml',
  cargoLock: 'apps/desktop/src-tauri/Cargo.lock',
};

async function fixture(t, { crlf = false } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'glassboard-version-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(join(root, 'scripts'), { recursive: true });
  await mkdir(join(root, 'apps/desktop/src-tauri'), { recursive: true });
  await mkdir(join(root, 'packages/ui'), { recursive: true });
  await copyFile(script, join(root, 'scripts/bump-version.mjs'));
  await writeFile(join(root, 'package.json'), JSON.stringify({ name: 'glassboard', private: true, workspaces: ['apps/*', 'packages/*'] }, null, 2) + '\n');
  await writeFile(join(root, 'packages/ui/package.json'), JSON.stringify({ name: '@glassboard/ui', version: '0.1.0' }, null, 2) + '\n');
  await writeFile(join(root, files.desktop), JSON.stringify({ name: '@glassboard/desktop', version: '0.1.0', private: true, dependencies: { '@glassboard/ui': '0.1.0' } }, null, 2) + '\n');
  await writeFile(join(root, files.npmLock), JSON.stringify({ name: 'glassboard', lockfileVersion: 3, requires: true, packages: { '': { name: 'glassboard', workspaces: ['apps/*', 'packages/*'] }, 'apps/desktop': { name: '@glassboard/desktop', version: '0.1.0', dependencies: { '@glassboard/ui': '0.1.0' } }, 'packages/ui': { name: '@glassboard/ui', version: '0.1.0' }, 'node_modules/@glassboard/desktop': { resolved: 'apps/desktop', link: true }, 'node_modules/@glassboard/ui': { resolved: 'packages/ui', link: true } } }, null, 2) + '\n');
  const nl = crlf ? '\r\n' : '\n';
  await writeFile(join(root, files.cargo), ['[package]', 'name = "glassboard"', 'version = "0.1.0"', '', '[dependencies]', 'example = "0.1.0"', ''].join(nl));
  await writeFile(join(root, files.cargoLock), ['version = 4', '', '[[package]]', 'name = "glassboard"', 'version = "0.1.0"', 'dependencies = ["example"]', '', '[[package]]', 'name = "example"', 'version = "0.1.0"', ''].join(nl));
  await writeFile(join(root, 'apps/desktop/src-tauri/tauri.conf.json'), JSON.stringify({ version: '../package.json' }) + '\n');
  return root;
}

async function snapshots(root) {
  return Object.fromEntries(await Promise.all(Object.entries(files).map(async ([key, path]) => [key, await readFile(join(root, path))])));
}

function run(root, argument, env = {}) {
  return spawnSync(process.execPath, [join(root, 'scripts/bump-version.mjs'), argument], {
    cwd: root, encoding: 'utf8', env: { ...process.env, ...env },
  });
}

async function assertVersions(root, expected) {
  const desktop = JSON.parse(await readFile(join(root, files.desktop), 'utf8'));
  const lock = JSON.parse(await readFile(join(root, files.npmLock), 'utf8'));
  assert.equal(desktop.version, expected);
  assert.equal(lock.packages['apps/desktop'].version, expected);
  assert.match(await readFile(join(root, files.cargo), 'utf8'), new RegExp(`^version = "${expected.replaceAll('.', '\\.')}"`, 'm'));
  assert.match(await readFile(join(root, files.cargoLock), 'utf8'), new RegExp(`^version = "${expected.replaceAll('.', '\\.')}"`, 'm'));
  assert.equal(run(root, '--check').status, 0);
}

test('native npm bumps patch, minor, major, and an explicit version', async t => {
  for (const [operation, expected] of [['patch', '0.1.1'], ['minor', '0.2.0'], ['major', '1.0.0'], ['0.2.0-rc.1', '0.2.0-rc.1']]) {
    await t.test(operation, async sub => {
      const root = await fixture(sub);
      const result = run(root, operation);
      assert.equal(result.status, 0, result.stderr);
      await assertVersions(root, expected);
      assert.match(await readFile(join(root, files.cargo), 'utf8'), /^example = "0\.1\.0"$/m);
      assert.match(await readFile(join(root, files.cargoLock), 'utf8'), /name = "example"\nversion = "0\.1\.0"/);
      assert.equal(JSON.parse(await readFile(join(root, files.desktop), 'utf8')).dependencies['@glassboard/ui'], '0.1.0');
      assert.equal(JSON.parse(await readFile(join(root, files.npmLock), 'utf8')).packages['packages/ui'].version, '0.1.0');
    });
  }
});

test('prerelease becomes stable and repeating an exact version is idempotent', async t => {
  const root = await fixture(t);
  assert.equal(run(root, '0.2.0-rc.1').status, 0);
  assert.equal(run(root, '0.2.0').status, 0);
  await assertVersions(root, '0.2.0');
  const before = await snapshots(root);
  assert.equal(run(root, '0.2.0').status, 0);
  assert.deepEqual(await snapshots(root), before);
});

test('command flags keep the lock current when inherited npm config disables it', async t => {
  const root = await fixture(t);
  const result = run(root, 'patch', { npm_config_package_lock: 'false', npm_config_workspaces_update: 'false' });
  assert.equal(result.status, 0, result.stderr);
  await assertVersions(root, '0.1.1');
});

test('validation rejects drift, malformed input, and a wrong Tauri pointer without writes', async t => {
  const root = await fixture(t);
  const before = await snapshots(root);
  for (const argument of ['--from-git', '--patch', '1.2', 'patch extra', '0.2.0-01', '0.2.0-rc.01', '0.2.0+build.1']) {
    const result = run(root, argument);
    assert.notEqual(result.status, 0);
    assert.doesNotMatch(result.stdout, /Version bumped:/);
    assert.deepEqual(await snapshots(root), before);
  }
  await writeFile(join(root, files.cargo), '[package]\nname = "glassboard"\nversion = "0.2.0"\n');
  const drifted = await snapshots(root);
  assert.notEqual(run(root, '--check').status, 0);
  assert.notEqual(run(root, 'patch').status, 0);
  assert.deepEqual(await snapshots(root), drifted);
  await writeFile(join(root, files.cargo), before.cargo);
  await writeFile(join(root, files.cargo), '[package]\nname = "glassboard"\nversion = "0.01.0"\n');
  const malformed = await snapshots(root);
  assert.notEqual(run(root, '--check').status, 0);
  assert.notEqual(run(root, 'patch').status, 0);
  assert.deepEqual(await snapshots(root), malformed);
  await writeFile(join(root, files.cargo), before.cargo);
  await writeFile(join(root, 'apps/desktop/src-tauri/tauri.conf.json'), '{"version":"0.1.0"}\n');
  assert.notEqual(run(root, 'patch').status, 0);
  assert.deepEqual(await snapshots(root), before);
});

test('failed npm child restores all four original files', async t => {
  const root = await fixture(t);
  const before = await snapshots(root);
  const fakeNpm = join(root, 'fake-npm.mjs');
  await writeFile(fakeNpm, 'import { writeFileSync } from "node:fs"; writeFileSync("apps/desktop/package.json", "partial"); writeFileSync("package-lock.json", "partial"); process.exit(7);\n');
  const result = run(root, 'patch', { npm_execpath: fakeNpm });
  assert.notEqual(result.status, 0);
  assert.deepEqual(await snapshots(root), before);
});

test('npm returning a different exact version restores all four original files', async t => {
  const root = await fixture(t);
  const before = await snapshots(root);
  const fakeNpm = join(root, 'fake-npm.mjs');
  await writeFile(fakeNpm, `import { readFileSync, writeFileSync } from 'node:fs';
for (const path of ['apps/desktop/package.json', 'package-lock.json']) {
  const data = JSON.parse(readFileSync(path, 'utf8'));
  if (path === 'package-lock.json') data.packages['apps/desktop'].version = '0.3.1';
  else data.version = '0.3.1';
  writeFileSync(path, JSON.stringify(data) + '\\n');
}
`);
  const result = run(root, '0.3.0', { npm_execpath: fakeNpm });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /npm returned 0\.3\.1 instead of requested version 0\.3\.0/);
  assert.doesNotMatch(result.stdout, /Version bumped:/);
  assert.deepEqual(await snapshots(root), before);
});

test('CRLF Rust files retain line endings; npm never changes Git refs', async t => {
  const root = await fixture(t, { crlf: true });
  const init = spawnSync('git', ['init', '-q'], { cwd: root });
  assert.equal(init.status, 0);
  const refs = () => spawnSync('git', ['show-ref'], { cwd: root, encoding: 'utf8' }).stdout;
  const beforeRefs = refs();
  assert.equal(run(root, 'patch').status, 0);
  await assertVersions(root, '0.1.1');
  for (const key of ['cargo', 'cargoLock']) {
    const source = await readFile(join(root, files[key]), 'utf8');
    assert.equal(source.replaceAll('\r\n', '').includes('\n'), false);
  }
  assert.equal(refs(), beforeRefs);
});
