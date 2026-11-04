import type { Catalog } from '@glassboard/ui/i18n';

const files = import.meta.glob<Catalog>('../../locales/*.json', { eager: true, import: 'default' });
/** The directory is the language list. Adding a language means adding a file. */
export const CATALOGS: Record<string, Catalog> = Object.fromEntries(Object.entries(files).map(([path, catalog]) => [path.slice(path.lastIndexOf('/') + 1, -'.json'.length), catalog]));
export const LANGUAGES = Object.entries(CATALOGS).map(([tag, catalog]) => ({ tag, name: catalog.$language })).sort((a, b) => a.name.localeCompare(b.name, 'en'));
