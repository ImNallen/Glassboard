import type { Catalog } from '@glassboard/ui/i18n';

// Vitest won't cache a file containing `import.meta.glob(`. A type argument hides that text, and the cached file then misses new catalogs.
const files = import.meta.glob('../../locales/*.json', { eager: true, import: 'default' }) as Record<string, Catalog>;
export const CATALOGS: Record<string, Catalog> = Object.fromEntries(Object.entries(files).map(([path, catalog]) => [path.slice(path.lastIndexOf('/') + 1, -'.json'.length), catalog]));
export const LANGUAGES = Object.entries(CATALOGS).map(([tag, catalog]) => ({ tag, name: catalog.$language })).sort((a, b) => a.name.localeCompare(b.name, 'en'));
