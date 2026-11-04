import { describe, expect, it } from 'vitest';
import { audit, loadProject } from '../../../../scripts/i18n.mjs';
import { LANGUAGES } from './languages';
import { pseudo } from './pseudo';

it('finds nothing wrong with the shipped sentences and catalogs', () => {
  expect(audit()).toEqual([]);
});

it('offers every catalog in the directory by its own name, and no pseudo catalog', () => {
  const { catalogs } = loadProject();
  expect(LANGUAGES.map(language => language.tag).sort()).toEqual(Object.keys(catalogs).sort());
  expect(LANGUAGES.find(language => language.tag === 'en')?.name).toBe('English');
  expect(LANGUAGES.some(language => language.tag === 'pseudo')).toBe(false);
});

describe('the audit reports what a contributor got wrong', () => {
  const messages = {
    'Press <kbd>{shortcut}</kbd>': '',
    'Quit Glassboard': '',
    Step: 'The numbered-marker tool.',
  };
  const good = { $language: 'Deutsch', 'Press <kbd>{shortcut}</kbd>': 'Drücke <kbd>{shortcut}</kbd>', 'Quit Glassboard': 'Glassboard beenden', Step: 'Schritt' };
  const code = `t('Press <kbd>{shortcut}</kbd>'); t('Quit Glassboard'); name: 'Step'`;
  const run = (de: Record<string, string>, text = code, notes: Record<string, string> = messages) =>
    audit({ messages: notes, catalogs: { en: { $language: 'English' }, de }, sources: [{ path: 'a.ts', text }] });

  it('passes a complete catalog', () => expect(run(good)).toEqual([]));
  it.each([
    ['a missing sentence', { ...good, Step: undefined }, 'de: missing: "Step"'],
    ['a stale sentence', { ...good, Old: 'Alt' }, 'de: stale: "Old"'],
    ['an empty translation', { ...good, Step: ' ' }, 'de: empty: "Step"'],
    ['a changed placeholder', { ...good, 'Press <kbd>{shortcut}</kbd>': 'Drücke <kbd>{taste}</kbd>' }, 'de: placeholder differs: "Press <kbd>{shortcut}</kbd>"'],
    ['a changed tag', { ...good, 'Press <kbd>{shortcut}</kbd>': 'Drücke {shortcut}' }, 'de: tag differs: "Press <kbd>{shortcut}</kbd>"'],
    ['a dropped brand name', { ...good, 'Quit Glassboard': 'Beenden' }, 'de: drops Glassboard: "Quit Glassboard"'],
    ['no language name', { ...good, $language: '' }, 'de: has no $language'],
  ])('%s', (_name, catalog, problem) => {
    expect(run(catalog as Record<string, string>)).toContain(problem);
  });
  it('a sentence no source file uses', () => {
    expect(run(good, `t('Quit Glassboard'); name: 'Step'`)).toContain('unused: "Press <kbd>{shortcut}</kbd>" is in messages.json but in no source file');
  });
  it('a one-word sentence with no note', () => {
    expect(run(good, code, { ...messages, Step: '' })).toContain('no note: "Step" is one word, so a translator needs its meaning');
  });
  it('Rust text that skips the macro or the list', () => {
    const rust = { path: 'x.rs', text: 'Text::message("a", vec![]); Text::untranslated("Sneaky");' };
    const problems = audit({ messages: {}, catalogs: { en: { $language: 'English' } }, sources: [rust] });
    expect(problems).toEqual(['bypass: x.rs calls Text::message instead of t!', 'untranslated: x.rs: "Sneaky" is not on the list in scripts/i18n.mjs']);
  });
});

it('pseudo-translates words and keeps placeholders and tags', () => {
  expect(pseudo('Press <kbd>{shortcut}</kbd> now')).toBe('⟦Press·⟧ <kbd>{shortcut}</kbd> ⟦now·⟧');
});
