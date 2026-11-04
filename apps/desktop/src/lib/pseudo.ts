import messages from '../../../../packages/ui/src/contract/messages.json';

export function pseudo(text: string) {
  return text.replace(/\{\w+\}|<\/?\w+>|\p{L}[\p{L}\p{M}'’-]*/gu, token => /^[{<]/.test(token) ? token : `⟦${token}${'·'.repeat(Math.ceil(token.length / 5))}⟧`);
}
export const pseudoCatalog = () => Object.fromEntries(Object.keys(messages).map(key => [key, pseudo(key)]));
