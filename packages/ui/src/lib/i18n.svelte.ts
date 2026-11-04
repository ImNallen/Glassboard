import type messages from '../contract/messages.json';

type Key = keyof typeof messages;
type Placeholder<S extends string> = S extends `${string}{${infer P}}${infer R}` ? P | Placeholder<R> : never;
type Tag<S extends string> = S extends `${string}<${infer T}>${infer R}` ? (T extends `/${string}` ? Tag<R> : T | Tag<R>) : never;
export type Args<K extends string> = Record<Placeholder<K>, string | number>;
/** A sentence holding markup, rendered through `parts` so a translator can move the tags. */
export type RichKey = Extract<Key, `${string}<${string}`>;
export type RichTag<K extends string> = Tag<K>;
type TextKey = Exclude<Key, RichKey>;
/** A sentence with no placeholders. Data tables hold these. */
export type Message = { [K in TextKey]: [Placeholder<K>] extends [never] ? K : never }[TextKey];
type Template = Exclude<TextKey, Message>;
export type Catalog = Record<string, string>;

let current = $state('en');
const catalogs: Record<string, Catalog> = {};

/** Only the desktop app calls this. A host that never does renders English from the keys. */
export function useCatalogs(next: Record<string, Catalog>) { Object.assign(catalogs, next); }
export function setLocale(next: string) { current = next; if (typeof document !== 'undefined') document.documentElement.lang = next; }
export function locale() { return current; }

const translated = (key: string) => catalogs[current]?.[key] ?? key;
function fill(template: string, args: Record<string, string | number> = {}) {
  return template.replace(/\{(\w+)\}/g, (placeholder, name) => {
    const value = args[name];
    return typeof value === 'number' ? number(value) : value ?? placeholder;
  });
}

export function t(key: Message): string;
export function t<K extends Template>(key: K, args: Args<K>): string;
export function t(key: string, args?: Record<string, string | number>) { return fill(translated(key), args); }

/** Splits a translated sentence on its tags, then fills each piece. Text never becomes HTML. */
export function parts<K extends RichKey>(key: K, args: Args<K>): { tag?: RichTag<K>; text: string }[] {
  const template = translated(key);
  const pieces: { tag?: RichTag<K>; text: string }[] = [];
  let end = 0;
  for (const match of template.matchAll(/<(\w+)>(.*?)<\/\1>/g)) {
    if (match.index > end) pieces.push({ text: fill(template.slice(end, match.index), args) });
    pieces.push({ tag: match[1] as RichTag<K>, text: fill(match[2], args) });
    end = match.index + match[0].length;
  }
  if (end < template.length) pieces.push({ text: fill(template.slice(end), args) });
  return pieces;
}
export function number(value: number, options?: Intl.NumberFormatOptions) { return new Intl.NumberFormat(current, options).format(value); }
export function list(items: string[], type: Intl.ListFormatType = 'conjunction') { return new Intl.ListFormat(current, { type }).format(items); }
