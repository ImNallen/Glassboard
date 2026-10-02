<script lang="ts">
  import { RotateCcw } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import { heldModifiers, MODIFIER_KEYS, platformMac, recordShortcut, sameShortcut, shortcutKeys, shortcutLabel } from '@glassboard/ui/shortcuts';
  import type { Errors } from '@glassboard/ui/errors';
  /**
   * A full-row button that records a new shortcut when clicked; `children` renders the row's icon and label.
   * The parent row must be positioned so the reset button can sit over its right edge. `physical` records key positions for the
   * native global shortcut; `clearable` lets Backspace or Delete unbind an in-app command.
   */
  let { label, shortcut, fallback, active, disabled = false, physical = false, clearable = false, onchange, errors, onrecord, children }: {
    label: string; children: Snippet; shortcut: string; fallback: string; active: boolean; disabled?: boolean; physical?: boolean; clearable?: boolean;
    onchange: (shortcut: string) => void; errors: Errors; onrecord?: (recording: boolean) => void;
  } = $props();
  let button: HTMLButtonElement;
  let recording = $state(false);
  let pending = $state<string[]>([]);
  let keys = $derived(shortcutKeys(recording ? pending.join('+') : shortcut, platformMac));
  let modified = $derived(!sameShortcut(shortcut, fallback, platformMac));
  $effect(() => { if (!active) stop(); });
  // WebKit doesn't focus buttons on click, so keys are read from the window while recording.
  function start() { recording = true; pending = []; errors.clear(); onrecord?.(true); button.focus(); }
  function stop() {
    if (!recording) return;
    recording = false; pending = []; onrecord?.(false);
  }
  function keydown(event: KeyboardEvent) {
    if (!recording) return;
    event.preventDefault(); event.stopPropagation();
    if (event.repeat) return;
    if (event.key === 'Escape') { stop(); return; }
    const bare = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;
    if (clearable && bare && (event.key === 'Backspace' || event.key === 'Delete')) { stop(); onchange(''); return; }
    if (MODIFIER_KEYS.has(event.key)) { pending = heldModifiers(event, platformMac); return; }
    const next = recordShortcut(event, platformMac, physical);
    if (!next) { errors.report('That key can’t be used in a shortcut.'); return; }
    stop();
    onchange(next);
  }
  function keyup(event: KeyboardEvent) {
    if (recording && MODIFIER_KEYS.has(event.key)) pending = heldModifiers(event, platformMac);
  }
  function pointerdown(event: PointerEvent) {
    if (recording && !button.contains(event.target as Node)) stop();
  }
</script>

<svelte:window onkeydowncapture={keydown} onkeyupcapture={keyup} onpointerdowncapture={pointerdown} onblur={stop}/>
<button bind:this={button} class="recorder" class:recording {disabled}
  aria-label={`${label}: ${recording ? 'press a new shortcut' : shortcut ? shortcutLabel(shortcut) : 'not set'}`}
  title={recording ? undefined : 'Click to change'}
  onclick={() => recording ? stop() : start()}>
  {@render children()}
  <span class="keys" class:unset={!recording && !keys.length}>
    {#each keys as key}<kbd>{key}</kbd>{/each}
    {#if recording}<span class="prompt">{keys.length ? '…' : 'Press keys'}</span>{:else if !keys.length}<span class="prompt">Not set</span>{/if}
  </span>
</button>
<button class="reset" disabled={disabled || !modified}
  title="Reset to default" aria-label={`Reset ${label} to ${shortcutLabel(fallback)}`} onclick={() => onchange(fallback)}><RotateCcw size={12} strokeWidth={2.2}/></button>

<style>
  .recorder { flex: 1; display: flex; align-items: center; gap: 10px; min-width: 0; min-height: 40px; padding: 5px 30px 5px 12px; text-align: left; transition: background .12s; }
  .recorder:hover:not(:disabled) { background: var(--gb-hover); }
  .recorder.recording { background: color-mix(in srgb, var(--gb-focus-ring) 7%, transparent); }
  .recorder:focus-visible { outline-offset: -2px; }
  .keys { display: inline-flex; align-items: center; justify-content: flex-end; gap: 3px; flex-shrink: 0; min-width: 72px; height: 28px; padding: 0 4px; border-radius: 8px; border: 1px solid transparent; transition: background .12s, border-color .12s, box-shadow .12s; }
  .recording .keys { background: var(--gb-input-surface); border-color: var(--gb-focus-ring); box-shadow: 0 0 0 3px color-mix(in srgb, var(--gb-focus-ring) 22%, transparent); }
  kbd { display: inline-grid; place-items: center; min-width: 21px; height: 21px; padding: 0 5px; border-radius: 5px; background: var(--keycap); box-shadow: 0 0 0 1px var(--gb-border), 0 1px 0 var(--gb-border); color: var(--gb-strong-text); font-size: 11px; font-weight: 500; line-height: 1; }
  .prompt { padding: 0 3px; font-size: 11px; color: var(--gb-muted); }
  .recording .prompt { color: var(--gb-focus-ring); animation: pulse 1.2s ease-in-out infinite; }
  .unset .prompt { font-style: italic; }
  .reset { position: absolute; top: 50%; right: 8px; translate: 0 -50%; display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; color: var(--reset); transition: background .12s, color .12s; }
  .reset:hover:not(:disabled) { background: color-mix(in srgb, var(--reset) 14%, transparent); }
  .reset:disabled { color: var(--gb-muted); opacity: .35; }
  @keyframes pulse { 50% { opacity: .45; } }
  @media (prefers-reduced-motion: reduce) { .recording .prompt { animation: none; } }
</style>
