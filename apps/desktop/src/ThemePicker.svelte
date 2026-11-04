<script lang="ts">
  import { t } from '@glassboard/ui/i18n';
  import { gradientStrip } from '@glassboard/ui/swatches';
  import type { ColorTheme } from '@glassboard/ui/themes';
  import Listbox from './Listbox.svelte';
  /** A dropdown of color themes, each previewed by its Rainbow strip and swatches. `selected` is undefined for custom colors. */
  let { themes, selected, custom, onselect }: {
    themes: readonly ColorTheme[]; selected: ColorTheme | undefined; custom: { colors: readonly string[]; swatches: readonly string[] };
    onselect: (theme: ColorTheme) => void;
  } = $props();
  let name = $derived(selected ? t(selected.name) : t('Custom'));
</script>

{#snippet preview(colors: readonly string[], swatches: readonly string[])}
  <span class="preview" aria-hidden="true">
    <span class="dots">{#each swatches as color}<span class="dot" style:background={color}></span>{/each}</span>
    <span class="strip" style:background={gradientStrip(colors)}></span>
  </span>
{/snippet}

<Listbox options={themes} {selected} id="theme-list" label={t('Color palette: {name}', { name })} listLabel={t('Color palettes')} {onselect}>
  {#snippet trigger()}
    <span class="name">{name}</span>
    {@render preview(selected?.colors ?? custom.colors, selected?.swatches ?? custom.swatches)}
  {/snippet}
  {#snippet option(theme: ColorTheme)}
    <span class="name">{t(theme.name)}</span>
    {@render preview(theme.colors, theme.swatches)}
  {/snippet}
</Listbox>

<style>
  .name { flex: 1; min-width: 0; font-size: 12.5px; font-weight: 500; color: var(--gb-text); }
  .preview { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
  .dots { display: flex; }
  .dot { width: 12px; height: 12px; margin-left: -3px; border-radius: 50%; box-shadow: 0 0 0 1.5px var(--gb-input-surface), inset 0 0 0 1px var(--gb-swatch-border); }
  .strip { width: 64px; height: 8px; border-radius: 4px; box-shadow: inset 0 0 0 1px var(--gb-swatch-border); }
  :global(.option) .dot { box-shadow: 0 0 0 1.5px var(--gb-surface), inset 0 0 0 1px var(--gb-swatch-border); }
  :global(.option.active) .dot { box-shadow: 0 0 0 1.5px color-mix(in srgb, var(--gb-surface), var(--gb-text) 5%), inset 0 0 0 1px var(--gb-swatch-border); }
</style>
