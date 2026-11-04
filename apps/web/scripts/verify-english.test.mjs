import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { filesWithCatalogs } from './verify-english.mjs';

test('finds a catalog anywhere under the build and nothing in an English one', () => {
  const dist = mkdtempSync(join(tmpdir(), 'glassboard-dist-'));
  try {
    mkdirSync(join(dist, '_astro'));
    writeFileSync(join(dist, 'index.html'), '<html lang="en">Hello</html>');
    writeFileSync(join(dist, '_astro', 'app.js'), 'const label = "Undo";');
    assert.deepEqual(filesWithCatalogs(dist), []);
    writeFileSync(join(dist, '_astro', 'de.js'), 'const de = {"$language":"Deutsch"};');
    assert.deepEqual(filesWithCatalogs(dist), [join('_astro', 'de.js')]);
  } finally {
    rmSync(dist, { recursive: true });
  }
});
