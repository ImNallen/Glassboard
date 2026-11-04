import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

/** Every catalog names itself with this key, and nothing else in the app uses it. */
const CATALOG_MARKER = '$language';

export function filesWithCatalogs(directory) {
  return readdirSync(directory, { recursive: true, withFileTypes: true })
    .filter(entry => entry.isFile())
    .map(entry => relative(directory, join(entry.parentPath, entry.name)))
    .filter(file => readFileSync(join(directory, file), 'latin1').includes(CATALOG_MARKER));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const directory = process.argv[2] ?? fileURLToPath(new URL('../dist', import.meta.url));
  const found = filesWithCatalogs(directory);
  if (found.length) {
    console.error(`The landing page must stay English, but these files carry a catalog:\n${found.join('\n')}`);
    process.exit(1);
  }
  console.log(`No catalogs in ${directory}.`);
}
