<script lang="ts">
  import { Check, ChevronDown } from '@lucide/svelte';
  import type { ColorTheme } from '@glassboard/ui/themes';
  /** A dropdown of color themes, each previewed by its Rainbow strip and swatches. `selected` is -1 for custom colors. */
  let { themes, selected, custom, onselect }: {
    themes: readonly ColorTheme[]; selected: number; custom: { colors: readonly string[]; swatches: readonly string[] };
    onselect: (theme: ColorTheme) => void;
  } = $props();
  let open = $state(false);
  let active = $state(0);
  let root: HTMLDivElement;
  let trigger: HTMLButtonElement;
  let list = $state<HTMLDivElement>();
  let current = $derived(selected >= 0 ? themes[selected] : null);
  const strip = (colors: readonly string[]) => `linear-gradient(90deg, ${colors.join(', ')})`;

  function show() {
    open = true;
    active = Math.max(selected, 0);
    queueMicrotask(() => list?.focus());
  }
  function close(refocus = true) {
    open = false;
    if (refocus) trigger.focus();
  }
  function choose(index: number) {
    close();
    if (index !== selected) onselect(themes[index]);
  }
  function triggerKeydown(event: KeyboardEvent) {
    if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) { event.preventDefault(); show(); }
  }
  function listKeydown(event: KeyboardEvent) {
    const moves: Record<string, number> = { ArrowDown: active + 1, ArrowUp: active - 1, Home: 0, End: themes.length - 1 };
    if (event.key in moves) { event.preventDefault(); active = Math.max(0, Math.min(themes.length - 1, moves[event.key])); }
    else if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); choose(active); }
    // Keep Escape from also closing the settings window.
    else if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(); }
    else if (event.key === 'Tab') close(false);
  }
  $effect(() => { if (open) list?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' }); });
</script>

<svelte:window onpointerdowncapture={event => { if (open && !root.contains(event.target as Node)) close(false); }}/>

{#snippet preview(colors: readonly string[], swatches: readonly string[])}
  <span class="preview" aria-hidden="true">
    <span class="dots">{#each swatches as color}<span class="dot" style:background={color}></span>{/each}</span>
    <span class="strip" style:background={strip(colors)}></span>
  </span>
{/snippet}

<div class="theme-picker" bind:this={root}>
  <button bind:this={trigger} class="trigger" class:open aria-haspopup="listbox" aria-expanded={open} aria-controls="theme-list"
    aria-label={`Theme: ${current?.name ?? 'Custom'}`} onclick={() => open ? close() : show()} onkeydown={triggerKeydown}>
    <span class="name">{current?.name ?? 'Custom'}</span>
    {@render preview(current?.colors ?? custom.colors, current?.swatches ?? custom.swatches)}
    <ChevronDown size={14} strokeWidth={2.2}/>
  </button>
  {#if open}
    <div bind:this={list} id="theme-list" class="options" role="listbox" tabindex="-1" aria-label="Themes"
      aria-activedescendant={`theme-${active}`} onkeydown={listKeydown}>
      {#each themes as theme, index}
        <!-- svelte-ignore a11y_click_events_have_key_events (the listbox handles keys) -->
        <div id={`theme-${index}`} class="option" class:active={index === active} role="option" aria-selected={index === selected} data-index={index}
          tabindex="-1" onpointerenter={() => active = index} onclick={() => choose(index)}>
          <span class="check">{#if index === selected}<Check size={13} strokeWidth={2.4}/>{/if}</span>
          <span class="name">{theme.name}</span>
          {@render preview(theme.colors, theme.swatches)}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .theme-picker { position: relative; }
  .trigger { display: flex; align-items: center; gap: 10px; width: 100%; height: 40px; padding: 0 10px 0 12px; border-radius: 10px; border: 1px solid var(--border); background: var(--input-surface); color: var(--muted); text-align: left; transition: border-color .12s, box-shadow .12s; }
  .trigger:hover { border-color: color-mix(in srgb, var(--text) 25%, transparent); }
  .trigger.open { border-color: var(--focus-ring); box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus-ring) 22%, transparent); }
  .trigger:focus-visible { outline-offset: 0; }
  .name { flex: 1; min-width: 0; font-size: 12.5px; font-weight: 500; color: var(--text); }
  .preview { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
  .dots { display: flex; }
  .dot { width: 12px; height: 12px; margin-left: -3px; border-radius: 50%; box-shadow: 0 0 0 1.5px var(--input-surface), inset 0 0 0 1px var(--swatch-border); }
  .strip { width: 64px; height: 8px; border-radius: 4px; box-shadow: inset 0 0 0 1px var(--swatch-border); }
  .options { position: absolute; z-index: 20; top: calc(100% + 6px); left: 0; right: 0; max-height: 260px; overflow-y: auto; padding: 4px; border-radius: 10px; border: 1px solid var(--border); background: var(--surface); box-shadow: 0 8px 24px var(--shadow), 0 2px 6px var(--shadow); outline: none; }
  .option { display: flex; align-items: center; gap: 8px; height: 36px; padding: 0 10px 0 6px; border-radius: 7px; cursor: pointer; }
  .option.active { background: var(--hover); }
  .option .dot { box-shadow: 0 0 0 1.5px var(--surface), inset 0 0 0 1px var(--swatch-border); }
  .option.active .dot { box-shadow: 0 0 0 1.5px color-mix(in srgb, var(--surface), var(--text) 5%), inset 0 0 0 1px var(--swatch-border); }
  .check { display: grid; place-items: center; width: 16px; color: var(--focus-ring); }
</style>
