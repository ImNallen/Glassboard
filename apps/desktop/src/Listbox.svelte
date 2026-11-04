<script lang="ts" generics="T">
  import { Check, ChevronDown } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  let { options, selected, id, label, listLabel, dividerAfter, trigger: triggerContent, option: optionContent, onselect }: {
    options: readonly T[]; selected: T | undefined; id: string; label: string; listLabel: string; dividerAfter?: number;
    trigger: Snippet; option: Snippet<[T]>; onselect: (option: T) => void;
  } = $props();
  let open = $state(false);
  let active = $state(0);
  let root: HTMLDivElement;
  let trigger: HTMLButtonElement;
  let list = $state<HTMLDivElement>();

  function show() {
    open = true;
    active = selected ? options.indexOf(selected) : 0;
    queueMicrotask(() => list?.focus());
  }
  function close(refocus = true) {
    open = false;
    if (refocus) trigger.focus();
  }
  function choose(index: number) {
    close();
    if (options[index] !== selected) onselect(options[index]);
  }
  function triggerKeydown(event: KeyboardEvent) {
    if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) { event.preventDefault(); show(); }
  }
  function listKeydown(event: KeyboardEvent) {
    const moves: Record<string, number> = { ArrowDown: active + 1, ArrowUp: active - 1, Home: 0, End: options.length - 1 };
    if (event.key in moves) { event.preventDefault(); active = Math.max(0, Math.min(options.length - 1, moves[event.key])); }
    else if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); choose(active); }
    // Keep Escape from also closing the settings window.
    else if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(); }
    else if (event.key === 'Tab') close(false);
  }
  $effect(() => { if (open) list?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' }); });
</script>

<svelte:window onpointerdowncapture={event => { if (open && !root.contains(event.target as Node)) close(false); }}/>

<div class="listbox" bind:this={root}>
  <button bind:this={trigger} class="trigger" class:open aria-haspopup="listbox" aria-expanded={open} aria-controls={id}
    aria-label={label} onclick={() => open ? close() : show()} onkeydown={triggerKeydown}>
    {@render triggerContent()}
    <ChevronDown size={14} strokeWidth={2.2}/>
  </button>
  {#if open}
    <div bind:this={list} {id} class="options" role="listbox" tabindex="-1" aria-label={listLabel}
      aria-activedescendant={`${id}-${active}`} onkeydown={listKeydown}>
      {#each options as option, index}
        <!-- svelte-ignore a11y_click_events_have_key_events (the listbox handles keys) -->
        <div id={`${id}-${index}`} class="option" class:active={index === active} role="option" aria-selected={option === selected} data-index={index}
          tabindex="-1" onpointerenter={() => active = index} onclick={() => choose(index)}>
          <span class="check">{#if option === selected}<Check size={13} strokeWidth={2.4}/>{/if}</span>
          {@render optionContent(option)}
        </div>
        {#if index === dividerAfter}<div class="divider" aria-hidden="true"></div>{/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .listbox { position: relative; }
  .trigger { display: flex; align-items: center; gap: 10px; width: 100%; height: 40px; padding: 0 10px 0 12px; border-radius: 10px; border: 1px solid var(--gb-border); background: var(--gb-input-surface); color: var(--gb-muted); text-align: left; transition: border-color .12s, box-shadow .12s; }
  .trigger:hover { border-color: color-mix(in srgb, var(--gb-text) 25%, transparent); }
  .trigger.open { border-color: var(--gb-focus-ring); box-shadow: 0 0 0 3px color-mix(in srgb, var(--gb-focus-ring) 22%, transparent); }
  .trigger:focus-visible { outline-offset: 0; }
  .options { position: absolute; z-index: 20; top: calc(100% + 6px); left: 0; right: 0; max-height: 260px; overflow-y: auto; padding: 4px; border-radius: 10px; border: 1px solid var(--gb-border); background: var(--gb-surface); box-shadow: 0 8px 24px var(--gb-shadow), 0 2px 6px var(--gb-shadow); outline: none; transform-origin: top center; animation: options-in 150ms var(--gb-ease-out); }
  .option { display: flex; align-items: center; gap: 8px; height: 36px; padding: 0 10px 0 6px; border-radius: 7px; cursor: pointer; }
  .option.active { background: var(--gb-hover); }
  .check { display: grid; place-items: center; width: 16px; color: var(--gb-focus-ring); }
  .divider { height: 1px; margin: 4px 6px; background: var(--gb-divider); }
  @keyframes options-in { from { opacity: 0; transform: scale(.97); } to { opacity: 1; transform: none; } }
  @keyframes options-fade { from { opacity: 0; } to { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .options { animation-name: options-fade; } }
</style>
