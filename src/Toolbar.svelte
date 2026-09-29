<script lang="ts">
  import { Pencil, ArrowUpRight, Square, Circle, Highlighter, Type, Eraser, MousePointer2, Undo2, Redo2, Trash2, Infinity as InfinityIcon, X } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { action, expandToolbar, native, savePreferences, shortcutLabel, toolbarProximity, type Preferences, type Session } from './lib/session';
  import { AUTO_FADE_OPTIONS, cycleColor, RAINBOW_PREVIEW, type Tool } from './lib/drawing';
  let { session, error, onerror }: { session: Session; error: string; onerror: (error: unknown) => void } = $props();
  let placement = $derived(session.preferences.toolbarPosition);
  let vertical = $derived(placement !== 'bottom');
  // Reveal within this distance of the visible toolbar (the native side applies the same margin to its window).
  const REVEAL_MARGIN = 20, COLLAPSE_DELAY = 250;
  let bar: HTMLDivElement;
  let near = $state(false);
  let focused = $state(false);
  // Starts as the pill; the toolbar only opens while something holds it open.
  let expanded = $state(false);
  let holdOpen = $derived(near || focused || Boolean(error || session.error));
  $effect(() => {
    if (holdOpen) { expanded = true; return; }
    const timer = setTimeout(() => expanded = false, COLLAPSE_DELAY);
    return () => clearTimeout(timer);
  });
  function measure(event: PointerEvent) {
    if (native || !bar) return;
    const rect = bar.getBoundingClientRect();
    near = event.clientX >= rect.left - REVEAL_MARGIN && event.clientX <= rect.right + REVEAL_MARGIN
      && event.clientY >= rect.top - REVEAL_MARGIN && event.clientY <= rect.bottom + REVEAL_MARGIN;
  }
  onMount(() => {
    let disposed = false, stop = () => {};
    toolbarProximity(value => near = value).then(fn => { if (disposed) fn(); else stop = fn; }).catch(onerror);
    return () => { disposed = true; stop(); };
  });
  let history = $derived(session.historyByOverlay[session.activeOverlay]);
  let colorMode = $derived(session.preferences.colorMode);
  let cyclePreview = $derived(`conic-gradient(${cycleColor(session.cycleIndex)} 0deg 120deg, ${cycleColor(session.cycleIndex + 2)} 120deg 240deg, ${cycleColor(session.cycleIndex + 4)} 240deg 360deg)`);
  const tools = [
    { id: 'pen', name: 'Pen', key: '1', icon: Pencil },
    { id: 'arrow', name: 'Arrow', key: '2', icon: ArrowUpRight },
    { id: 'rectangle', name: 'Square', key: '3', icon: Square },
    { id: 'ellipse', name: 'Circle', key: '4', icon: Circle },
    { id: 'highlighter', name: 'Highlight', key: '5', icon: Highlighter },
    { id: 'text', name: 'Text', key: '6', icon: Type },
    { id: 'eraser', name: 'Eraser', key: '7', icon: Eraser },
  ] as const;
  const colors = [['#000000', 'Black'], ['#ffffff', 'White'], [cycleColor(2), 'Green'], [cycleColor(1), 'Yellow'], [cycleColor(0), 'Red'], [cycleColor(4), 'Blue']] as const;
  const widths = [{ value: 2, name: 'Thin' }, { value: 4, name: 'Regular' }, { value: 7, name: 'Bold' }] as const;
  let widthIndex = $derived(widths.findIndex(width => width.value === session.preferences.width));
  let currentWidth = $derived(widths[widthIndex] ?? widths[1]);
  let nextWidth = $derived(widths[(widthIndex + 1) % widths.length]);
  let fadeSeconds = $derived(session.preferences.autoFadeSeconds);
  let nextFade = $derived(AUTO_FADE_OPTIONS[(AUTO_FADE_OPTIONS.indexOf(fadeSeconds) + 1) % AUTO_FADE_OPTIONS.length]);
  const fadeLabel = (seconds: number) => seconds === 0 ? 'Keep forever' : `${seconds}s`;
  const run = (name: string) => action(name).catch(onerror);
  async function change(patch: Partial<Preferences>) {
    try { await savePreferences({ ...session.preferences, ...patch }); }
    catch (e) { onerror(e); }
  }
  async function choose(tool: Tool) {
    await change({ tool });
    if (session.mode === 'hidden') await run('show');
    else if (session.mode !== 'draw') await run('interact');
  }
  $effect(() => { expandToolbar(Boolean(error || session.error)).catch(onerror); });
</script>

<svelte:window onpointermove={measure} onmouseout={event => { if (!native && !event.relatedTarget) near = false; }} />
<div class="toolbar-host" class:vertical class:left={placement === 'left'} class:right={placement === 'right'} class:collapsed={!expanded} class:concealed={session.mode === 'hidden'} onfocusin={() => focused = true} onfocusout={event => { focused = event.currentTarget.contains(event.relatedTarget as Node); }}>
  <button class="pill" tabindex="-1" aria-hidden={expanded} title="Show toolbar" onclick={() => { near = true; expanded = true; }} onpointerenter={() => near = true}></button>
  {#if error || session.error}
    <div class="error-banner" role="alert"><span>{error || session.error}</span><button aria-label="Dismiss error" onclick={() => { onerror(''); run('dismiss-error'); }}><X size={14}/></button></div>
  {/if}
  <div class="toolbar" bind:this={bar} role="toolbar" aria-label="Annotation tools" aria-orientation={vertical ? 'vertical' : 'horizontal'}>
    <div class="tool-row">
    <div class="tool-group">{#each tools as tool}<button class="icon-button tool" class:active={session.preferences.tool === tool.id && session.mode === 'draw'} onclick={() => choose(tool.id)} title={`${tool.name} (${shortcutLabel(`CommandOrControl+${tool.key}`)})`} aria-label={tool.name} aria-pressed={session.preferences.tool === tool.id && session.mode === 'draw'}><tool.icon size={19} strokeWidth={1.8}/></button>{/each}</div>
    <div class="divider"></div>
    <div class="toolbar-colors" role="group" aria-label="Drawing color">
      <button class="icon-button color-choice" class:active={colorMode === 'rainbow'} aria-pressed={colorMode === 'rainbow'} title="Rainbow · changes as you draw" aria-label="Rainbow" onclick={() => change({ colorMode: 'rainbow' })}><span class="color-dot" style:background={RAINBOW_PREVIEW}></span></button>
      <button class="icon-button color-choice" class:active={colorMode === 'cycle'} aria-pressed={colorMode === 'cycle'} title="Shifting colors · cycles through 7 colors, one per shape" aria-label="Shifting colors: a new color for each shape" onclick={() => change({ colorMode: 'cycle' })}><span class="color-dot" style:background={cyclePreview} aria-hidden="true"></span></button>
      {#each colors as [color, name]}
        <button class="icon-button color-choice" class:active={colorMode === 'solid' && session.preferences.color === color} aria-pressed={colorMode === 'solid' && session.preferences.color === color} title={name} aria-label={name} onclick={() => change({ color, colorMode: 'solid' })}><span class="color-dot" style:background={color}></span></button>
      {/each}
    </div>
    <button class="icon-button" onclick={() => change({ width: nextWidth.value })} title={`Line width: ${currentWidth.name} · click for ${nextWidth.name}`} aria-label={`Line width: ${currentWidth.name}. Switch to ${nextWidth.name}`}><span class="stroke-sample" style:height={`${currentWidth.value}px`}></span></button>
    <div class="divider"></div>
    <button class="icon-button fade-button" class:active={fadeSeconds !== 0} onclick={() => change({ autoFadeSeconds: nextFade })} title={`Auto-fade: ${fadeLabel(fadeSeconds)} · click for ${fadeLabel(nextFade)}`} aria-label={`Auto-fade: ${fadeLabel(fadeSeconds)}. Switch to ${fadeLabel(nextFade)}`}>
      {#if fadeSeconds === 0}<InfinityIcon size={19}/>{:else}<span>{fadeSeconds}s</span>{/if}
    </button>
    <button class="icon-button" class:active={session.mode === 'interact'} onclick={() => run('interact')} title="Interact with screen (V)" aria-label="Interact with screen" aria-pressed={session.mode === 'interact'}><MousePointer2 size={19}/></button>
    <button class="icon-button" disabled={!history?.canUndo} onclick={() => run('undo')} title={history?.canUndo ? 'Undo (⌘/Ctrl+Z)' : 'Nothing to undo'} aria-label="Undo"><Undo2 size={18}/></button>
    <button class="icon-button" disabled={!history?.canRedo} onclick={() => run('redo')} title={history?.canRedo ? 'Redo (⌘/Ctrl+Shift+Z)' : 'Nothing to redo'} aria-label="Redo"><Redo2 size={18}/></button>
    <button class="icon-button" onclick={() => run('clear')} title="Clear this display" aria-label="Clear this display"><Trash2 size={17}/></button>
    </div>

  </div>
</div>
