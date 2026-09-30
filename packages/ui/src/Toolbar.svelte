<script lang="ts">
  import { Pencil, ArrowUpRight, Square, Circle, Highlighter, Type, Eraser, Scan, Undo2, Redo2, Trash2, Infinity as InfinityIcon, X } from '@lucide/svelte';
  import { onMount, tick } from 'svelte';
  import { createToolbarHints } from './lib/toolbar-hints.svelte';
  import { COLOR_SHORTCUTS, TOOL_SHORTCUTS } from './lib/shortcuts';
  import { action, expandToolbar, native, savePreferences, shortcutLabel, toolbarPointer, type Preferences, type Session } from './lib/session';
  import { AUTO_FADE_OPTIONS, cycleColor, RAINBOW_PREVIEW, type Tool } from './lib/drawing';
  /** `pinned` keeps the toolbar open instead of collapsing to the pill when the cursor is away; hosts without a native window use it. */
  let { session, error, onerror, pinned = false, embedded = false, capture = false, selectingCapture = false, oncapture, onchoose, onsave = savePreferences }: { session: Session; error: string; onerror: (error: unknown) => void; pinned?: boolean; embedded?: boolean; capture?: boolean; selectingCapture?: boolean; oncapture?: () => void; onchoose?: () => void; onsave?: (preferences: Preferences) => Promise<void> } = $props();
  let placement = $derived(session.preferences.toolbarPosition);
  let vertical = $derived(placement !== 'bottom');
  // Reveal within this distance of the visible toolbar (the native side applies the same margin to its window).
  const REVEAL_MARGIN = 20, COLLAPSE_DELAY = 250;
  let bar: HTMLDivElement;
  let host: HTMLDivElement;
  let tooltip = $state<HTMLDivElement>();
  let near = $state(false);
  let focused = $state(false);
  // Starts as the pill; the toolbar only opens while something holds it open.
  let expanded = $state(false);
  const hints = createToolbarHints({
    get drawing() { return session.mode === 'draw'; },
    get expanded() { return expanded; },
    get placement() { return placement; },
    get host() { return host; },
    get tooltip() { return tooltip; },
  });
  const { hint } = hints;
  let holdOpen = $derived(pinned || near || focused || Boolean(hints.tip) || session.tutorial === 'draw' || session.tutorial === 'hide' || Boolean(error || session.error));
  $effect(() => {
    if (holdOpen) { expanded = true; return; }
    const timer = setTimeout(() => expanded = false, COLLAPSE_DELAY);
    return () => clearTimeout(timer);
  });
  function measure(event: PointerEvent) {
    if ((native && !embedded) || !bar) return;
    const rect = bar.getBoundingClientRect();
    near = event.clientX >= rect.left - REVEAL_MARGIN && event.clientX <= rect.right + REVEAL_MARGIN
      && event.clientY >= rect.top - REVEAL_MARGIN && event.clientY <= rect.bottom + REVEAL_MARGIN;
  }
  onMount(() => {
    let disposed = false, stop = () => {};
    if (embedded) return () => hints.dismiss();
    toolbarPointer(async pointer => {
      near = pointer?.near ?? false;
      // Wait for a collapsed toolbar to reveal before hit-testing its buttons.
      await tick();
      if (disposed) return;
      const target = pointer && expanded && session.mode === 'draw'
        ? document.elementFromPoint(pointer.x, pointer.y) : null;
      hints.hoverTarget(target);
    }).then(fn => { if (disposed) fn(); else stop = fn; }).catch(onerror);
    return () => { disposed = true; stop(); hints.dismiss(); };
  });
  let history = $derived(selectingCapture ? undefined : session.historyByOverlay[session.activeOverlay]);
  let colorMode = $derived(session.preferences.colorMode);
  let cyclePreview = $derived(`conic-gradient(${cycleColor(session.cycleIndex)} 0deg 120deg, ${cycleColor(session.cycleIndex + 2)} 120deg 240deg, ${cycleColor(session.cycleIndex + 4)} 240deg 360deg)`);
  const toolIcons = { arrow: ArrowUpRight, pen: Pencil, rectangle: Square, ellipse: Circle, eraser: Eraser, text: Type, highlighter: Highlighter };
  const tools = TOOL_SHORTCUTS.map(tool => ({ ...tool, icon: toolIcons[tool.id] }));
  const colors = COLOR_SHORTCUTS.filter(choice => choice.colorMode === 'solid');
  let fadeSeconds = $derived(session.preferences.autoFadeSeconds);
  let nextFade = $derived(AUTO_FADE_OPTIONS[(AUTO_FADE_OPTIONS.indexOf(fadeSeconds) + 1) % AUTO_FADE_OPTIONS.length]);
  const fadeLabel = (seconds: number) => seconds === 0 ? 'Until you return to work' : `${seconds}s`;
  const run = (name: string) => action(name).catch(onerror);
  async function change(patch: Partial<Preferences>) {
    try { await onsave({ ...session.preferences, ...patch }); return true; }
    catch (e) { onerror(e); return false; }
  }
  async function choose(tool: Tool) {
    if (!await change({ tool })) return;
    onchoose?.();
    if (session.mode === 'hidden') await run('show');
  }
  $effect(() => { if (embedded) return; expandToolbar(Boolean(error || session.error || hints.tip)).catch(onerror); });
</script>

<svelte:window onresize={hints.position} onblur={hints.blur} onpointermove={measure} onmouseout={event => { if ((!native || embedded) && !event.relatedTarget) near = false; }} />
<div bind:this={host} class="toolbar-host" class:vertical class:left={placement === 'left'} class:right={placement === 'right'} class:collapsed={!expanded} class:concealed={session.mode === 'hidden'} onfocusin={() => focused = true} onfocusout={event => { focused = event.currentTarget.contains(event.relatedTarget as Node); }}>
  <button class="pill" tabindex="-1" aria-hidden={expanded} title="Show toolbar" onclick={() => { near = true; expanded = true; }} onpointerenter={() => near = true}></button>
  {#if error || session.error}
    <div class="error-banner" role="alert"><span>{error || session.error}</span><button aria-label="Dismiss error" onclick={() => { onerror(''); run('dismiss-error'); }}><X size={14}/></button></div>
  {/if}
  {#if hints.tip && !error && !session.error}
    <div bind:this={tooltip} id="toolbar-tooltip" class="toolbar-tooltip" class:ready={hints.ready} role="tooltip"
      style:left={!vertical ? `${hints.offset}px` : undefined} style:top={vertical ? `${hints.offset}px` : undefined}
      onpointerenter={hints.keepOpen} onpointerleave={hints.hideSoon}>
      <div class="tooltip-heading"><span>{hints.tip.label}</span>{#if hints.tip.key}<kbd>{hints.tip.key}</kbd>{/if}</div>
      {#if hints.tip.description}<div class="tooltip-description">{hints.tip.description}</div>{/if}
    </div>
  {/if}
  <div class="toolbar" bind:this={bar} onscroll={hints.dismiss} role="toolbar" aria-label="Annotation tools" aria-orientation={vertical ? 'vertical' : 'horizontal'}>
    <div class="tool-row">
    <div class="tool-group">{#each tools as tool}<button class="icon-button tool" class:active={!selectingCapture && session.preferences.tool === tool.id && session.mode === 'draw'} onclick={() => choose(tool.id)} use:hint={{ label: tool.name, key: shortcutLabel(`CommandOrControl+${tool.key}`) }} aria-label={tool.name} aria-pressed={!selectingCapture && session.preferences.tool === tool.id && session.mode === 'draw'}><tool.icon size={19} strokeWidth={1.8}/></button>{/each}
      {#if oncapture}<button class="icon-button tool" class:active={selectingCapture} aria-pressed={selectingCapture} aria-label="Screenshot" onclick={oncapture} use:hint={{ label: 'Screenshot', key: shortcutLabel('CommandOrControl+S'), description: 'Drag to select or replace the capture area' }}><Scan size={19} strokeWidth={1.8}/></button>{/if}
    </div>
    <div class="divider"></div>
    <div class="toolbar-colors" role="group" aria-label="Drawing color">
      <button class="icon-button color-choice" class:active={colorMode === 'rainbow'} aria-pressed={colorMode === 'rainbow'} use:hint={{ label: 'Rainbow', key: '1', description: 'Changes as you draw' }} aria-label="Rainbow" onclick={() => change({ colorMode: 'rainbow' })}><span class="color-dot" style:background={RAINBOW_PREVIEW}></span></button>
      <button class="icon-button color-choice" class:active={colorMode === 'cycle'} aria-pressed={colorMode === 'cycle'} use:hint={{ label: 'Shifting colors', key: '2', description: 'A new color for each shape' }} aria-label="Shifting colors: a new color for each shape" onclick={() => change({ colorMode: 'cycle' })}><span class="color-dot" style:background={cyclePreview} aria-hidden="true"></span></button>
      {#each colors as { color, name, key }}
        <button class="icon-button color-choice" class:active={colorMode === 'solid' && session.preferences.color === color} aria-pressed={colorMode === 'solid' && session.preferences.color === color} use:hint={{ label: name, key }} aria-label={name} onclick={() => change({ color, colorMode: 'solid' })}><span class="color-dot" style:background={color}></span></button>
      {/each}
    </div>
    <div class="divider"></div>
    {#if !capture}<button class="icon-button fade-button" class:active={fadeSeconds !== 0} onclick={() => change({ autoFadeSeconds: nextFade })} use:hint={{ label: `Auto-fade: ${fadeLabel(fadeSeconds)}`, description: `Click for ${fadeLabel(nextFade)}` }} aria-label={`Auto-fade: ${fadeLabel(fadeSeconds)}. Switch to ${fadeLabel(nextFade)}`}>
      {#if fadeSeconds === 0}<InfinityIcon size={19}/>{:else}<span>{fadeSeconds}s</span>{/if}
    </button>
    {/if}
    <button class="icon-button" disabled={!history?.canUndo} onclick={() => run('undo')} use:hint={{ label: history?.canUndo ? 'Undo' : 'Nothing to undo', key: shortcutLabel('CommandOrControl+Z') }} aria-label="Undo"><Undo2 size={18}/></button>
    <button class="icon-button" disabled={!history?.canRedo} onclick={() => run('redo')} use:hint={{ label: history?.canRedo ? 'Redo' : 'Nothing to redo', key: shortcutLabel('CommandOrControl+Shift+Z') }} aria-label="Redo"><Redo2 size={18}/></button>
    <button class="icon-button" disabled={selectingCapture} onclick={() => run('clear')} use:hint={{ label: 'Clear this display' }} aria-label="Clear this display"><Trash2 size={17}/></button>
    </div>

  </div>
</div>

<style>
  .toolbar-tooltip { position: absolute; bottom: calc(100% + 8px); z-index: 30; width: max-content; max-width: min(280px, calc(100vw - 24px)); padding: 9px 11px; border: 1px solid var(--border); border-radius: 9px; background: var(--surface); color: var(--text); box-shadow: 0 3px 10px var(--shadow); font-size: 12px; line-height: 1.4; visibility: hidden; }
  .toolbar-tooltip.ready { visibility: visible; }
  .tooltip-heading { display: flex; align-items: center; justify-content: space-between; gap: 16px; font-weight: 500; }
  .tooltip-heading kbd { flex-shrink: 0; font-weight: 400; }
  .tooltip-description { margin-top: 4px; font-size: 11px; color: var(--muted); }
  .vertical .toolbar-tooltip { left: calc(100% + 10px); bottom: auto; }
  .right .toolbar-tooltip { left: auto; right: calc(100% + 10px); }
</style>
