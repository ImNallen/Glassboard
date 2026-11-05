import { afterEach, describe, expect, it } from 'vitest';
import { list, locale, number, setLocale, t, useCatalogs } from './i18n.svelte';
import { createPreviewSession, defaults, savePreferences, subscribe, useSession } from './session';

useCatalogs({
  de: {
    Small: 'Klein',
    'Size: {size}': 'Größe: {size}',
    'Size: {size}. Switch to {next}': 'Wechsle von {size} zu {next}',
  },
});
afterEach(() => setLocale('en'));

describe('t', () => {
  it('renders the English key when no catalog is registered for the language', () => {
    expect(t('Small')).toBe('Small');
    expect(t('Size: {size}', { size: 'Small' })).toBe('Size: Small');
    setLocale('ko');
    expect(t('Small')).toBe('Small');
  });
  it('lets a translation reorder its placeholders', () => {
    setLocale('de');
    expect(t('Small')).toBe('Klein');
    expect(t('Size: {size}. Switch to {next}', { size: 'Klein', next: 'Groß' })).toBe('Wechsle von Klein zu Groß');
  });
  it('falls back to English for a sentence the catalog lacks', () => {
    setLocale('de');
    expect(t('Medium')).toBe('Medium');
  });
  it('does not expand a placeholder that an argument contains', () => {
    expect(t('Size: {size}. Switch to {next}', { size: '{next}', next: 'Large' })).toBe('Size: {next}. Switch to Large');
  });
  it('formats number arguments for the language', () => {
    setLocale('de');
    expect(t('Size: {size}', { size: 1234.5 })).toBe('Größe: 1.234,5');
  });
});

it('follows the language for numbers and lists', () => {
  expect(number(0.123, { style: 'percent', minimumFractionDigits: 1 })).toBe('12.3%');
  expect(list(['A', 'B', 'C'], 'disjunction')).toBe('A, B, or C');
  setLocale('de');
  expect(number(1.5)).toBe('1,5');
  expect(list(['A', 'B', 'C'])).toBe('A, B und C');
});

it('takes the language from each session snapshot, not from English defaults', async () => {
  useSession(createPreviewSession({ preferences: { ...defaults.preferences, language: 'de' } }));
  const stop = await subscribe(() => {});
  expect(locale()).toBe('de');
  await savePreferences({ ...defaults.preferences, language: 'system' });
  expect(locale()).toBe('en');
  stop();
});
