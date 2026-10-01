<script lang="ts">
  import { Plus, RotateCcw } from '@lucide/svelte';
  import { shortcutLabel, type Preferences } from '@glassboard/ui/session';
  import { COLOR_SHORTCUTS, keybinding } from '@glassboard/ui/shortcuts';
  import { CYCLE_COLORS, rainbowPreview, shiftingPreview } from '@glassboard/ui/drawing';
  import { DEFAULT_SWATCHES, defaultSwatches, sameColor, sameColors, SEQUENCE_LIMITS, swatchColor, swatchName, SWATCH_PRESETS } from '@glassboard/ui/swatches';
  import { COLOR_THEMES, THEMED_SWATCHES_START, type ColorTheme } from '@glassboard/ui/themes';
  import ColorPicker from './ColorPicker.svelte';
  import ThemePicker from './ThemePicker.svelte';
  /** The Colors tab: apply a theme, or replace any swatch and the Rainbow and Shifting color lists. */
  let { preferences, onsave }: { preferences: Preferences; onsave: (patch: Partial<Preferences>, message?: string) => void } = $props();
  type Sequence = 'rainbowColors' | 'cycleColors';
  /** A swatch slot, or one of the color lists. */
  type Target = number | Sequence;
  const SLOTS = DEFAULT_SWATCHES.map((_, slot) => slot);
  const SEQUENCES: readonly { key: Sequence; name: string; command: string; description: string }[] = [
    { key: 'rainbowColors', name: 'Rainbow', command: 'color-rainbow', description: 'Each mark blends through these colors.' },
    { key: 'cycleColors', name: 'Shifting', command: 'color-cycle', description: 'Each new mark takes the next color.' },
  ];
  let editing = $state<Target>(0);
  // The selected color within a list.
  let chip = $state(0);
  // A color being dragged in the picker, shown before it's saved.
  let draft = $state<string | null>(null);
  // A chip being dragged to a new position. Pointer events, not HTML drag and drop, which webviews handle inconsistently.
  let drag = $state<{ from: number; to: number; pointer: number; x: number; y: number; moved: boolean } | null>(null);
  let chipList = $state<HTMLDivElement>();
  // The click that follows a drag must not select the chip's old position.
  let dropped = false;
  let swatches = $derived(SLOTS.map(slot => swatchColor(preferences.swatches, slot)));
  let sequence = $derived(typeof editing === 'string' ? SEQUENCES.find(item => item.key === editing)! : null);
  let list = $derived(sequence ? preferences[sequence.key] : []);
  let index = $derived(Math.min(chip, Math.max(list.length - 1, 0)));
  let current = $derived(typeof editing === 'number' ? swatches[editing] : list[index]);
  let shown = $derived(draft ?? current);
  let shownSwatches = $derived(SLOTS.map(slot => slot === editing && draft ? draft : swatches[slot]));
  let editedList = $derived(list.map((color, i) => i === index && draft ? draft : color));
  // While dragging, show the list as it would be after dropping.
  let shownList = $derived(drag?.moved ? reorder(editedList, drag.from, drag.to) : editedList);
  let shownIndex = $derived(drag?.moved ? drag.to : index);
  let name = $derived(typeof editing === 'number' ? swatchName(preferences.swatches, editing) : sequence!.name);
  let swatchModified = $derived(SLOTS.map(slot => !sameColor(swatches[slot], DEFAULT_SWATCHES[slot].color)));
  let listModified = $derived({ rainbowColors: !sameColors(preferences.rainbowColors, CYCLE_COLORS), cycleColors: !sameColors(preferences.cycleColors, CYCLE_COLORS) });
  let modified = $derived(typeof editing === 'number' ? swatchModified[editing] : listModified[editing]);
  let themed = $derived(swatches.slice(THEMED_SWATCHES_START));
  let theme = $derived(COLOR_THEMES.findIndex(item => sameColors(item.colors, preferences.rainbowColors)
    && sameColors(item.colors, preferences.cycleColors) && sameColors(item.swatches, themed)));
  $effect(() => { if (draft && sameColor(draft, current)) draft = null; });
  const keyLabel = (command: string) => shortcutLabel(keybinding(preferences.keybindings, command));
  const slotCommand = (slot: number) => COLOR_SHORTCUTS.find(choice => choice.slot === slot)!.command;
  const strip = (colors: readonly string[]) => `linear-gradient(90deg, ${colors.join(', ')})`;
  const segments = (colors: readonly string[]) => `linear-gradient(90deg, ${colors.map((color, i) => `${color} ${i / colors.length * 100}% ${(i + 1) / colors.length * 100}%`).join(', ')})`;

  function choose(target: Target) { editing = target; chip = 0; draft = null; }
  /** Save swatches; a selected solid color follows its swatch so the toolbar keeps it chosen. */
  function saveSwatches(next: string[], patch: Partial<Preferences> = {}, message?: string) {
    const following = preferences.colorMode === 'solid' ? swatches.findIndex(color => sameColor(color, preferences.color)) : -1;
    onsave({ ...patch, swatches: next, ...(following >= 0 ? { color: next[following] } : {}) }, message);
  }
  function saveList(next: string[], select = index) {
    if (!sequence) return;
    draft = null;
    chip = select;
    onsave({ [sequence.key]: next });
  }
  function setColor(color: string) {
    if (sameColor(color, current)) { draft = null; return; }
    if (typeof editing === 'number') {
      const slot = editing;
      saveSwatches(swatches.map((previous, i) => i === slot ? color : previous));
    } else saveList(list.map((previous, i) => i === index ? color : previous));
  }
  function add() {
    if (list.length >= SEQUENCE_LIMITS.max) return;
    const color = SWATCH_PRESETS.find(preset => !list.some(existing => sameColor(existing, preset))) ?? list[index];
    saveList([...list.slice(0, index + 1), color, ...list.slice(index + 1)], index + 1);
  }
  function remove(at = index) {
    if (list.length <= SEQUENCE_LIMITS.min) return;
    saveList(list.filter((_, i) => i !== at), Math.min(at, list.length - 2));
  }
  function reorder(colors: readonly string[], from: number, to: number) {
    const next = [...colors];
    next.splice(to, 0, ...next.splice(from, 1));
    return next;
  }
  function move(from: number, to: number) {
    if (from === to || to < 0 || to >= list.length) return;
    saveList(reorder(list, from, to), to);
  }
  function dragStart(event: PointerEvent, at: number) {
    if (event.button !== 0) return;
    // Capture keeps moves coming while the pointer is over other chips; dragging still works without it.
    try { (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId); } catch { /* No active pointer to capture. */ }
    drag = { from: at, to: at, pointer: event.pointerId, x: event.clientX, y: event.clientY, moved: false };
  }
  function dragMove(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointer || !chipList) return;
    if (!drag.moved && Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < 4) return;
    // The nearest chip slot to the pointer is where the color will land.
    const centers = [...chipList.querySelectorAll('.chip')].map(chip => {
      const rect = chip.getBoundingClientRect();
      return Math.hypot(event.clientX - (rect.left + rect.width / 2), event.clientY - (rect.top + rect.height / 2));
    });
    drag = { ...drag, moved: true, to: centers.indexOf(Math.min(...centers)) };
  }
  function dragEnd(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointer) return;
    const { from, to, moved } = drag;
    drag = null;
    if (moved) { dropped = true; move(from, to); }
  }
  function chipClick(at: number) {
    if (dropped) { dropped = false; return; }
    chip = at; draft = null;
  }
  function chipKeydown(event: KeyboardEvent, at: number) {
    const step = event.key === 'ArrowLeft' ? -1 : event.key === 'ArrowRight' ? 1 : 0;
    if (step && event.altKey) { event.preventDefault(); move(at, at + step); }
    else if (step) { event.preventDefault(); chip = Math.max(0, Math.min(list.length - 1, at + step)); draft = null; }
    else if (event.key === 'Backspace' || event.key === 'Delete') { event.preventDefault(); remove(at); }
    else return;
    // Keep keyboard focus on the chip that was moved or selected.
    queueMicrotask(() => document.querySelector<HTMLButtonElement>(`.chip[data-index="${chip}"]`)?.focus());
  }
  function reset() {
    draft = null;
    if (typeof editing === 'number') setColor(DEFAULT_SWATCHES[editing].color);
    else saveList([...CYCLE_COLORS], 0);
  }
  function applyTheme(next: ColorTheme) {
    draft = null; chip = 0;
    saveSwatches([...swatches.slice(0, THEMED_SWATCHES_START), ...next.swatches],
      { rainbowColors: [...next.colors], cycleColors: [...next.colors] }, `${next.name} theme applied.`);
  }
  function resetAll() {
    draft = null; chip = 0;
    saveSwatches(defaultSwatches(), { rainbowColors: [...CYCLE_COLORS], cycleColors: [...CYCLE_COLORS] }, 'Colors reset.');
  }
</script>

<div class="intro">
  <p>Pick a theme, then tweak any color.</p>
  <button class="text-button reset-all" disabled={!swatchModified.some(Boolean) && !listModified.rainbowColors && !listModified.cycleColors} onclick={resetAll}>Reset all<RotateCcw size={12}/></button>
</div>

<section class="group" aria-labelledby="theme-title">
  <h2 id="theme-title">Theme</h2>
  <ThemePicker themes={COLOR_THEMES} selected={theme} custom={{ colors: preferences.rainbowColors, swatches: themed }} onselect={applyTheme}/>
</section>

<section class="group" aria-labelledby="toolbar-title">
  <h2 id="toolbar-title">Toolbar</h2>
  <div class="toolbar-preview" role="radiogroup" aria-labelledby="toolbar-title">
    {#each SEQUENCES as item}
      <button class="slot" class:editing={editing === item.key} role="radio" aria-checked={editing === item.key}
        aria-label={`${item.name}, key ${keyLabel(item.command) || 'not set'}`} onclick={() => choose(item.key)}>
        <span class="dot" style:background={item.key === 'rainbowColors' ? rainbowPreview(editing === item.key ? shownList : preferences.rainbowColors)
          : shiftingPreview(editing === item.key ? shownList : preferences.cycleColors)}></span>
        <span class="key">{keyLabel(item.command)}</span>
      </button>
    {/each}
    <span class="divider" aria-hidden="true"></span>
    {#each SLOTS as slot}
      <button class="slot" class:editing={editing === slot} role="radio" aria-checked={editing === slot}
        aria-label={`${swatchName(preferences.swatches, slot)}, key ${keyLabel(slotCommand(slot)) || 'not set'}`} onclick={() => choose(slot)}>
        <span class="dot" style:background={shownSwatches[slot]}></span><span class="key">{keyLabel(slotCommand(slot))}</span>
      </button>
    {/each}
  </div>
</section>

<section class="group" aria-labelledby="edit-title">
  <div class="edit-heading">
    <h2 id="edit-title">{name}</h2>
    <button class="reset" disabled={!modified} title="Reset to default" aria-label={`Reset ${name} to default`} onclick={reset}><RotateCcw size={12} strokeWidth={2.2}/></button>
  </div>

  {#if sequence}
    <p class="note">{sequence.description}</p>
    <div class="sequence-preview" aria-hidden="true" style:background={sequence.key === 'rainbowColors' ? strip([...shownList, shownList[0]]) : segments(shownList)}></div>
    <div class="chips">
      <div bind:this={chipList} class="chip-list" role="listbox" aria-label={`${sequence.name} colors`} aria-orientation="horizontal">
        {#each shownList as color, i (i)}
          <button class="chip" class:selected={i === shownIndex} class:dragging={drag?.moved && drag.to === i} role="option" aria-selected={i === shownIndex} data-index={i}
            tabindex={i === index ? 0 : -1} style:background={color} title={color.toUpperCase()}
            aria-label={`Color ${i + 1} of ${shownList.length}, ${color.toUpperCase()}`}
            onclick={() => chipClick(i)} onkeydown={event => chipKeydown(event, i)} onpointerdown={event => dragStart(event, i)} onpointermove={dragMove}
            onpointerup={dragEnd} onpointercancel={() => drag = null}></button>
        {/each}
      </div>
      <button class="add" disabled={list.length >= SEQUENCE_LIMITS.max} title="Add a color" aria-label="Add a color" onclick={add}><Plus size={14} strokeWidth={2.2}/></button>
    </div>
    <div class="chip-actions">
      <span>Drag to reorder · {list.length} of {SEQUENCE_LIMITS.max}</span>
      <button class="text-button" disabled={list.length <= SEQUENCE_LIMITS.min} onclick={() => remove()}>Remove color</button>
    </div>

    <h3>Color {index + 1}</h3>
  {/if}

  <div class="presets" role="group" aria-label="Suggested colors">
    {#each SWATCH_PRESETS as preset}
      <button class="preset" class:chosen={sameColor(preset, shown)} aria-pressed={sameColor(preset, shown)} title={preset.toUpperCase()}
        aria-label={`Use ${preset.toUpperCase()}`} style:background={preset} onclick={() => { draft = null; setColor(preset); }}></button>
    {/each}
  </div>
  {#key `${editing}-${index}`}
    <ColorPicker value={shown} label={sequence ? `${name} color ${index + 1}` : name} oninput={color => draft = color} onchange={setColor}/>
  {/key}
</section>

<style>
  .toolbar-preview { display: flex; align-items: flex-start; justify-content: center; gap: 2px; padding: 8px 8px 5px; border: 1px solid var(--border); border-radius: 14px; background: var(--surface); box-shadow: 0 2px 6px var(--shadow); }
  .slot { display: flex; flex-direction: column; align-items: center; gap: 4px; width: 36px; padding: 5px 0 3px; border-radius: 9px; transition: background .12s, box-shadow .12s; }
  .slot:hover { background: var(--hover); }
  .slot.editing { background: var(--selected); box-shadow: inset 0 0 0 1px var(--selected-border); }
  .dot { width: 22px; height: 22px; border-radius: 50%; box-shadow: inset 0 0 0 1px var(--swatch-border); transition: background .12s; }
  .key { height: 13px; font-size: 10px; font-weight: 500; line-height: 13px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .editing .key { color: var(--strong-text); }
  .divider { align-self: center; width: 1px; height: 22px; margin: 0 4px 14px; background: var(--divider); }

  .edit-heading { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  .edit-heading h2 { margin-bottom: 0; }
  .reset { display: grid; place-items: center; width: 22px; height: 22px; margin-right: 7px; border-radius: 6px; color: var(--reset); transition: background .12s; }
  .reset:hover:not(:disabled) { background: color-mix(in srgb, var(--reset) 14%, transparent); }
  .reset:disabled { color: var(--muted); opacity: .35; }
  .note { margin: -2px 2px 10px; font-size: 11px; color: var(--muted); }
  h3 { margin: 16px 2px 8px; font-size: 10px; font-weight: 600; letter-spacing: .6px; text-transform: uppercase; color: var(--muted); }

  .sequence-preview { height: 10px; margin-bottom: 10px; border-radius: 5px; box-shadow: inset 0 0 0 1px var(--swatch-border); }
  .chips { display: flex; align-items: center; gap: 8px; padding: 9px 10px; border: 1px solid var(--divider); border-radius: 10px; background: var(--subtle-surface); }
  .chip-list { display: flex; flex-wrap: wrap; gap: 8px; flex: 1; }
  .chip { width: 26px; height: 26px; padding: 0; border-radius: 50%; cursor: grab; touch-action: none; box-shadow: inset 0 0 0 1px var(--swatch-border); transition: transform .12s, box-shadow .12s, opacity .12s; }
  .chip:hover { transform: scale(1.1); }
  .chip.selected { box-shadow: inset 0 0 0 1px var(--swatch-border), 0 0 0 2px var(--surface), 0 0 0 4px var(--focus-ring); }
  .chip.dragging { transform: scale(1.18); cursor: grabbing; box-shadow: inset 0 0 0 1px var(--swatch-border), 0 0 0 2px var(--surface), 0 0 0 4px var(--focus-ring), 0 4px 10px var(--shadow); }
  .add { display: grid; place-items: center; flex-shrink: 0; width: 26px; height: 26px; border-radius: 50%; color: var(--secondary-text); box-shadow: inset 0 0 0 1px var(--border); border: 1px dashed transparent; }
  .add:hover:not(:disabled) { background: var(--hover); color: var(--strong-text); }
  .chip-actions { display: flex; align-items: center; justify-content: space-between; margin: 6px 2px 0; font-size: 10.5px; color: var(--muted); }
  .chip-actions .text-button { margin-right: -6px; }


  .presets { display: grid; grid-template-columns: repeat(8, 1fr); gap: 8px; margin-bottom: 14px; padding: 10px; border: 1px solid var(--divider); border-radius: 10px; background: var(--subtle-surface); }
  .preset { justify-self: center; width: 26px; height: 26px; padding: 0; border-radius: 50%; box-shadow: inset 0 0 0 1px var(--swatch-border); transition: transform .12s, box-shadow .12s; }
  .preset:hover { transform: scale(1.1); }
  .preset.chosen { box-shadow: inset 0 0 0 1px var(--swatch-border), 0 0 0 2px var(--surface), 0 0 0 4px var(--focus-ring); }
</style>
