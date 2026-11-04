import messages from '../../../../packages/ui/src/contract/messages.json';

/** Wraps each word so a sweep can spot text that skipped `t`, padded to about 1.5 times English to stress layout. */
export function pseudo(text: string) {
  return text.replace(/\{\w+\}|<\/?\w+>|\p{L}[\p{L}\p{M}'’-]*/gu, token => /^[{<]/.test(token) ? token : `⟦${token}${'·'.repeat(Math.ceil(token.length / 5))}⟧`);
}
export const pseudoCatalog = () => Object.fromEntries(Object.keys(messages).map(key => [key, pseudo(key)]));
