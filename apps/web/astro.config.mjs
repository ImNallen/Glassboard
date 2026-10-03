// @ts-check
import { loadEnv } from 'vite';
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

const { PAGES_BASE_URL } = loadEnv('production', '.', 'PAGES_');
const deployment = new URL(PAGES_BASE_URL || 'https://glassboard.dev');
export default defineConfig({
  site: deployment.origin,
  base: deployment.pathname,
  trailingSlash: 'always',
  integrations: [svelte()],
  // Astro's dev toolbar docks at the bottom center, on top of the Glassboard toolbar.
  devToolbar: { enabled: false },
});
