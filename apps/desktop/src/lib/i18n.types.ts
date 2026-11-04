import type { Snippet } from 'svelte';
import Rich from '../Rich.svelte';

declare const snippet: Snippet<[string]>;
const kbdSentence = 'Undo with <kbd>{shortcut}</kbd>.';
const twoTags = 'Another app is using <kbd>{shortcut}</kbd>. <button>Choose a different shortcut</button>';

export function richMistakes() {
  Rich(null as never, { message: kbdSentence, args: { shortcut: 'A' }, kbd: snippet });
  Rich(null as never, { message: twoTags, args: { shortcut: 'A' }, kbd: snippet, button: snippet });
  // @ts-expect-error the kbd snippet is missing
  Rich(null as never, { message: kbdSentence, args: { shortcut: 'A' } });
  // @ts-expect-error the button snippet is missing
  Rich(null as never, { message: twoTags, args: { shortcut: 'A' }, kbd: snippet });
  // @ts-expect-error a mistyped placeholder
  Rich(null as never, { message: kbdSentence, args: { shortcuts: 'A' }, kbd: snippet });
  // @ts-expect-error a sentence with no markup
  Rich(null as never, { message: 'Undo', args: {} });
}
