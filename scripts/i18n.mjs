// @ts-nocheck catalogs.test.ts imports this into a webview project, which has no Node types.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const ROOT = join(import.meta.dirname, '..');
const LOCALES = join(ROOT, 'apps/desktop/locales');
const MESSAGES = join(ROOT, 'packages/ui/src/contract/messages.json');
const SOURCE_ROOTS = ['packages/ui/src', 'apps/desktop/src', 'apps/desktop/src-tauri/src'];
const BRAND = 'Glassboard';
const UNTRANSLATED = new Set([
  'Capture cancelled',
  'Expected a binary capture image',
  'Only drawing windows can report history',
  'Only the toolbar can resize itself',
  'Preference saving is unavailable',
  'Preference saving stopped',
  'The capture ID is invalid',
  'The selected capture region is invalid',
]);

const placeholders = text => (text.match(/\{\w+\}/g) ?? []).sort().join();
const tags = text => (text.match(/<\/?\w+>/g) ?? []).sort().join();

function sourceFiles(directory) {
  return readdirSync(join(ROOT, directory), { recursive: true, encoding: 'utf8' })
    .filter(file => /\.(ts|svelte|rs)$/.test(file) && !/\.test\.ts$|i18n\.types\.ts$/.test(file))
    .map(file => ({ path: join(directory, file), text: readFileSync(join(ROOT, directory, file), 'utf8') }));
}

function withoutRustTests(text) {
  const start = text.search(/#\[cfg\(test\)\]\s*mod tests \{/);
  if (start < 0) return text;
  let depth = 0;
  for (let at = text.indexOf('{', start); at < text.length; at++) {
    if (text[at] === '{') depth++;
    else if (text[at] === '}' && --depth === 0) return text.slice(0, start) + withoutRustTests(text.slice(at + 1));
  }
  return text.slice(0, start);
}

export function loadProject() {
  return {
    messages: JSON.parse(readFileSync(MESSAGES, 'utf8')),
    catalogs: Object.fromEntries(readdirSync(LOCALES).filter(file => file.endsWith('.json')).sort()
      .map(file => [file.slice(0, -'.json'.length), JSON.parse(readFileSync(join(LOCALES, file), 'utf8'))])),
    sources: SOURCE_ROOTS.flatMap(sourceFiles).map(({ path, text }) => ({ path, text: path.endsWith('.rs') ? withoutRustTests(text) : text })),
  };
}

function isLanguageTag(tag) {
  try { return Intl.getCanonicalLocales(tag)[0] === tag; }
  catch { return false; }
}

function catalogProblems(tag, catalog, messages) {
  const problems = [];
  if (!isLanguageTag(tag)) problems.push('file name is not a canonical language tag');
  if (!catalog.$language) problems.push('has no $language');
  const entries = Object.entries(catalog).filter(([key]) => key !== '$language');
  for (const [key] of entries) if (!(key in messages)) problems.push(`stale: ${JSON.stringify(key)}`);
  if (tag === 'en') {
    for (const [key] of entries) if (key in messages) problems.push(`overrides its own key: ${JSON.stringify(key)}`);
    return problems;
  }
  for (const key of Object.keys(messages)) {
    const value = catalog[key];
    if (value === undefined) { problems.push(`missing: ${JSON.stringify(key)}`); continue; }
    if (!value.trim()) problems.push(`empty: ${JSON.stringify(key)}`);
    if (placeholders(value) !== placeholders(key)) problems.push(`placeholder differs: ${JSON.stringify(key)}`);
    if (tags(value) !== tags(key)) problems.push(`tag differs: ${JSON.stringify(key)}`);
    if (key.includes(BRAND) && !value.includes(BRAND)) problems.push(`drops ${BRAND}: ${JSON.stringify(key)}`);
  }
  return problems;
}

export function audit({ messages, catalogs, sources } = loadProject()) {
  const problems = [];
  for (const [tag, catalog] of Object.entries(catalogs)) {
    for (const problem of catalogProblems(tag, catalog, messages)) problems.push(`${tag}: ${problem}`);
  }
  if (!catalogs.en) problems.push('en: apps/desktop/locales/en.json is missing');
  const code = sources.map(source => source.text).join('\n');
  for (const [key, note] of Object.entries(messages)) {
    if (!['"', "'", '`'].some(quote => code.includes(`${quote}${key}${quote}`))) problems.push(`unused: ${JSON.stringify(key)} is in messages.json but in no source file`);
    if (!/\s/.test(key) && !note) problems.push(`no note: ${JSON.stringify(key)} is one word, so a translator needs its meaning`);
  }
  for (const { path, text } of sources) {
    if (!path.endsWith('.rs')) continue;
    if (!path.endsWith('i18n.rs') && text.includes('Text::message(')) problems.push(`bypass: ${path} calls Text::message instead of t!`);
    for (const [, literal] of text.matchAll(/Text::untranslated\(\s*"([^"]*)"/g)) {
      if (!UNTRANSLATED.has(literal)) problems.push(`untranslated: ${path}: ${JSON.stringify(literal)} is not on the list in scripts/i18n.mjs`);
    }
  }
  return problems;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const project = loadProject();
  const problems = audit(project);
  const total = Object.keys(project.messages).length;
  for (const [tag, catalog] of Object.entries(project.catalogs)) {
    const done = Object.keys(project.messages).filter(key => catalog[key]).length;
    console.log(tag.padEnd(10), tag === 'en' ? `source, ${total} sentences` : `${done}/${total}`);
  }
  for (const problem of problems) console.log(problem);
  process.exitCode = problems.length ? 1 : 0;
}
