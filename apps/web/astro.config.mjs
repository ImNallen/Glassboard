// @ts-check
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

// https://astro.build/config
export default defineConfig({
  site: 'https://glassboard.dev',
  integrations: [svelte()],
  // Astro's dev toolbar docks at the bottom center, on top of the Glassboard toolbar.
  devToolbar: { enabled: false },
});
