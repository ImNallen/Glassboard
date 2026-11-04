import { t, type Message } from './i18n.svelte';

export function mistakes(anyString: string) {
  // @ts-expect-error a mistyped key
  t('Smal');
  // @ts-expect-error a sentence that is not in messages.json
  t('Not a sentence we ship');
  // @ts-expect-error a key that takes placeholders, called without them
  t('Size: {size}');
  // @ts-expect-error a mistyped placeholder
  t('Size: {size}', { szie: 'Small' });
  // @ts-expect-error an extra placeholder
  t('Size: {size}', { size: 'Small', next: 'Large' });
  // @ts-expect-error a missing placeholder
  t('Size: {size}. Switch to {next}', { size: 'Small' });
  // @ts-expect-error a plain string instead of a literal sentence
  t(anyString);
  // @ts-expect-error a sentence with markup belongs to Rich
  t('Undo with <kbd>{shortcut}</kbd>.', { shortcut: 'Z' });
  // @ts-expect-error a table entry that is not a sentence
  const name: Message = 'Smal';
  return name;
}
