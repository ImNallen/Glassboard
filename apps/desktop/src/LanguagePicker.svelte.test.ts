// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { setLocale, useCatalogs } from '@glassboard/ui/i18n';
import { createPreviewSession, defaults, subscribe, useSession, type Session } from '@glassboard/ui/session';
import { createErrors } from '@glassboard/ui/errors';
import Settings from './Settings.svelte';
import { LANGUAGES } from './lib/languages';
import { watchUpdates, type UpdateStatus } from './lib/updates';

vi.mock('./lib/updates', async original => {
  const updates = await original<typeof import('./lib/updates')>();
  return { ...updates, watchUpdates: vi.fn(updates.watchUpdates) };
});

useCatalogs({ de: {
  Undo: 'Rückgängig', Language: 'Sprache', 'System default': 'Systemstandard', 'Language: {language}': 'Sprache: {language}', Ctrl: 'Strg',
  'Glassboard on GitHub': 'Glassboard auf GitHub',
} });

let settings: ReturnType<typeof mount> | undefined;
let stop = () => {};
let session: () => Session = () => defaults;
beforeEach(() => { Element.prototype.scrollIntoView = () => {}; });
afterEach(async () => {
  if (settings) await unmount(settings);
  settings = undefined;
  stop();
  document.body.replaceChildren();
  vi.restoreAllMocks();
  setLocale('en');
});

async function setup(language = 'system', tab = 'general') {
  useSession(createPreviewSession({ preferences: { ...defaults.preferences, language } }));
  let current = $state<Session>(structuredClone(defaults));
  stop = await subscribe(value => current = value);
  session = () => current;
  settings = mount(Settings, { target: document.body, props: { get session() { return current; }, errors: createErrors(() => current) } });
  await tick();
  document.querySelector<HTMLElement>(`#tab-${tab}`)!.click();
  flushSync();
}
const trigger = () => document.querySelector<HTMLButtonElement>('button[aria-controls="language-list"]')!;
const options = () => [...document.querySelectorAll<HTMLElement>('#language-list [role="option"]')];
const heading = () => document.querySelector('#language-title')!;
async function openList() { trigger().click(); await tick(); flushSync(); }

it('is the first group on the General tab, and names itself in English when the language is not', async () => {
  await setup();
  expect(document.querySelector('.panel .group')?.getAttribute('aria-labelledby')).toBe('language-title');
  expect(heading().querySelector('svg')).not.toBeNull();
  expect(heading().querySelector('small')).toBeNull();
  await unmount(settings!); settings = undefined; stop();
  await setup('de');
  expect(heading().textContent).toBe('SpracheLanguage');
  expect(heading().querySelector('small')?.textContent).toBe('Language');
  expect(heading().querySelector('small')?.getAttribute('lang')).toBe('en');
});

it('offers the system default and then each language by its own name, marked with its language', async () => {
  await setup();
  await openList();
  expect(options().map(option => option.textContent?.trim())).toEqual(['System default', ...LANGUAGES.map(language => language.name)]);
  expect(options()[0].querySelector('[lang]')).toBeNull();
  expect(options().slice(1).map(option => option.querySelector('[lang]')?.getAttribute('lang'))).toEqual(LANGUAGES.map(language => language.tag));
  expect(document.querySelectorAll('#language-list .divider')).toHaveLength(1);
});

it('applies a choice at once and returns focus to the trigger', async () => {
  await setup();
  await openList();
  options().find(option => option.textContent?.trim() === 'Deutsch')!.click();
  await tick(); flushSync();
  expect(session().preferences.language).toBe('de');
  expect(session().locale).toBe('de');
  expect(document.querySelector('#language-list')).toBeNull();
  expect(document.activeElement).toBe(trigger());
  expect(trigger().textContent?.trim()).toBe('Deutsch');
  expect(document.documentElement.lang).toBe('de');
});

it('chooses by keyboard, and Escape closes only the list', async () => {
  await setup('en');
  trigger().dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }));
  await tick(); flushSync();
  const list = document.querySelector<HTMLElement>('#language-list')!;
  list.dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', bubbles: true, cancelable: true }));
  flushSync();
  list.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
  await tick(); flushSync();
  expect(session().preferences.language).toBe('system');
  await openList();
  const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
  document.querySelector<HTMLElement>('#language-list')!.dispatchEvent(escape);
  flushSync();
  expect(document.querySelector('#language-list')).toBeNull();
  expect(session().settingsOpen).toBe(false);
});

it('shows the system default for a saved language this build has no catalog for, and keeps the saved tag', async () => {
  await setup('tlh');
  expect(trigger().textContent?.trim()).toBe('System default');
  expect(session().preferences.language).toBe('tlh');
});

it('rewords itself when the language changes, and the page language follows', async () => {
  await setup();
  expect(trigger().getAttribute('aria-label')).toBe('Language: System default');
  await unmount(settings!); settings = undefined; stop();
  await setup('de');
  expect(trigger().getAttribute('aria-label')).toBe('Sprache: Deutsch');
  expect(document.documentElement.lang).toBe('de');
  await openList();
  expect(options()[0].textContent?.trim()).toBe('Systemstandard');
});

it('searches what the user sees in the current language, folding case and accents', async () => {
  await setup('de', 'keybindings');
  const search = document.querySelector<HTMLInputElement>('input[type="search"]')!;
  const rows = (value: string) => {
    search.value = value;
    search.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    return document.querySelectorAll('button.recorder').length;
  };
  expect(rows('ruckgangig')).toBe(1);
  expect(rows('RÜCKGÄNGIG')).toBe(1);
  expect(rows('undo')).toBe(0);
  expect(rows('strg')).toBeGreaterThan(1);
});

it('keeps a failed update on screen when a language change republishes it', async () => {
  let emit: (status: UpdateStatus) => void = () => {};
  vi.mocked(watchUpdates).mockImplementation(async fn => { emit = fn; fn({ state: 'idle' }); return () => {}; });
  await setup();
  await tick();
  const line = () => document.querySelector('.status p')?.textContent;
  emit({ state: 'installing', version: '0.2.0' });
  emit({ state: 'failed', message: 'Could not install the update.' });
  flushSync();
  expect(line()).toBe('Could not install the update.');
  emit({ state: 'failed', message: 'Das Update konnte nicht installiert werden.' });
  flushSync();
  expect(line()).toBe('Das Update konnte nicht installiert werden.');
  emit({ state: 'checking' });
  flushSync();
  expect(line()).toBeUndefined();
  emit({ state: 'failed', message: 'A background check failed.' });
  flushSync();
  expect(line()).toBeUndefined();
});
