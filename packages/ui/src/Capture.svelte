<script lang="ts">
  import { onMount } from 'svelte';
  import { Copy } from '@lucide/svelte';
  import Overlay from '@glassboard/ui/Overlay.svelte';
  import Toolbar from '@glassboard/ui/Toolbar.svelte';
  import { action, native, savePreferences, shortcutLabel, type Session, type Preferences } from '@glassboard/ui/session';
  import { drawingKeydown } from '@glassboard/ui/keys';
  import { toolShortcut } from '@glassboard/ui/shortcuts';
  import { annotatedCapture, captureKeydown, captureRegion, type CaptureRegion } from '@glassboard/ui/capture';

  let { session, onerror, getImage, copyImage }: { session: Session; onerror: (error: unknown) => void; getImage: (id: number) => Promise<ImageData | HTMLImageElement | HTMLCanvasElement>; copyImage: (id: number, image: Promise<Blob>) => Promise<void> } = $props();
  // The parent keys this editor by capture id.
  const id = (() => session.capture!.id)();
  let image: HTMLCanvasElement;
  let viewportWidth = $state(0);
  let viewportHeight = $state(0);
  let loaded = $state(false);
  let error = $state('');
  let busy = $state(false);
  let region = $state<CaptureRegion | null>(null);
  let selecting = $state(true);
  let dragging = $state(false);
  let start: { x: number; y: number } | null = null;
  let pointer: number | null = null;
  let overlay = $state<Overlay>();
  let disposed = false;
  let editingSession = $derived<Session>({ ...session, mode: 'draw', tutorial: null, settingsOpen: false,
    activeOverlay: 'capture', preferences: { ...session.preferences, autoFadeSeconds: 0 } });
  const run = (name: string) => action(name).catch(onerror);
  function fail(cause: unknown) { error = String(cause); }
  function cancel() { if (!busy) run('cancel-capture'); }
  function begin(event: PointerEvent) {
    if (!loaded || event.button !== 0 || pointer !== null) return;
    event.preventDefault();
    pointer = event.pointerId;
    start = { x: event.clientX, y: event.clientY };
    region = { ...start, width: 0, height: 0 };
    dragging = true;
    (event.currentTarget as HTMLElement).setPointerCapture(pointer);
  }
  function move(event: PointerEvent) {
    if (event.pointerId !== pointer || !start) return;
    region = captureRegion(start, { x: event.clientX, y: event.clientY }, window.innerWidth, window.innerHeight);
  }
  function finish(event: PointerEvent) {
    if (event.pointerId !== pointer) return;
    move(event);
    dragging = false; start = null; pointer = null;
    if (!region || region.width < 8 || region.height < 8) region = null;
  }
  function abandon() { dragging = false; start = null; pointer = null; region = null; }
  function reselect() { if (busy) return; selecting = true; error = ''; }
  function chooseDrawingTool() {
    if (region && !dragging) selecting = false;
    else run('show');
  }
  async function copy() {
    if (busy || dragging || !region || !image || !loaded) return;
    busy = true; error = '';
    try {
      await copyImage(id, annotatedCapture(image, region, overlay?.exportShapes() ?? []));
      if (!native && !disposed && session.capture?.id === id) await action('cancel-capture');
    } catch (cause) { if (!disposed) fail(cause); }
    finally { busy = false; }
  }
  function save(preferences: Preferences) {
    return savePreferences({ ...session.preferences, tool: preferences.tool, color: preferences.color, colorMode: preferences.colorMode });
  }
  function keydown(event: KeyboardEvent) {
    captureKeydown(event, { copy, cancel });
    if (event.defaultPrevented || dragging || busy) return;
    const choosingTool = Boolean(toolShortcut(event));
    drawingKeydown(event, editingSession, { run, capture: reselect, save: preferences => save(preferences).then(() => {
      if (choosingTool) chooseDrawingTool();
    }).catch(fail) });
  }
  function copied(event: ClipboardEvent) {
    const target = event.target instanceof Element ? event.target : document.activeElement;
    if (target?.closest('input, textarea, [contenteditable="true"]')) return;
    event.preventDefault(); copy();
  }
  onMount(() => {
    getImage(id).then(value => {
      if (disposed) return;
      image.width = value instanceof HTMLImageElement ? value.naturalWidth : value.width;
      image.height = value instanceof HTMLImageElement ? value.naturalHeight : value.height;
      const context = image.getContext('2d');
      if (!context) throw new Error('Could not display the screenshot. Cancel and try again.');
      if (value instanceof HTMLImageElement || value instanceof HTMLCanvasElement) context.drawImage(value, 0, 0);
      else context.putImageData(value, 0, 0);
      loaded = true;
    }).catch(cause => { if (!disposed) fail(cause); });
    return () => { disposed = true; image.width = 0; image.height = 0; };
  });
</script>

<svelte:window bind:innerWidth={viewportWidth} bind:innerHeight={viewportHeight} onkeydown={keydown} oncopy={copied}/>
<main class="capture-editor" style:width={`${viewportWidth}px`} style:height={`${viewportHeight}px`} class:ready={loaded} class:unavailable={!loaded && Boolean(error)} aria-label="Screenshot capture">
  <canvas class="capture-image" bind:this={image} aria-label="Frozen screenshot"></canvas>
  <div class="capture-dim" aria-hidden="true"></div>
  {#if selecting}
    <div class="selection-surface" role="application" aria-label="Drag to select a screenshot region" onpointerdown={begin} onpointermove={move} onpointerup={finish} onpointercancel={abandon}>
      {#if region}<div class="selection" style:left={`${region.x}px`} style:top={`${region.y}px`} style:width={`${region.width}px`} style:height={`${region.height}px`}></div>{/if}
    </div>
  {:else if region}
    <div class="selection finished" style:left={`${region.x}px`} style:top={`${region.y}px`} style:width={`${region.width}px`} style:height={`${region.height}px`}></div>
    <Overlay bind:this={overlay} session={editingSession} onerror={fail} bounds={region} showGlow={false}/>
  {/if}
  <div class:busy class="capture-tools"><Toolbar session={editingSession} error="" onerror={fail} onsave={save} onchoose={chooseDrawingTool} oncapture={reselect} selectingCapture={selecting} pinned embedded capture/></div>
  {#if region && !dragging}
    <div class="capture-actions" role="group" aria-label="Capture controls">
      <button class="copy" disabled={busy || !loaded} onclick={copy} title={shortcutLabel('CommandOrControl+C')}><Copy size={16}/>{busy ? 'Copying…' : 'Copy & close'}<kbd>{shortcutLabel('CommandOrControl+C')}</kbd></button>
    </div>
  {/if}
  {#if error}<div class="capture-error" role="alert">{error}</div>{/if}
</main>

<style>
  /* A reserved scrollbar gutter also reduces vw units. Use the measured viewport
     so the displayed screenshot and annotation coordinates share one size. */
  .capture-editor { position: fixed; inset: 0; color-scheme: light dark; }
  .capture-editor > :global(canvas) { width: inherit; height: inherit; }
  .capture-image { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
  /* Keep the frozen pixels still; ease only the dimming and editor controls in.
     A separate dim layer avoids replaying the entrance when reselecting a region. */
  .capture-dim { position: absolute; inset: 0; pointer-events: none; background: rgb(0 0 0 / .18); opacity: 0; }
  .capture-tools, .capture-actions { opacity: 0; pointer-events: none; }
  .ready .capture-dim, .ready .capture-tools, .ready .capture-actions { opacity: 1; animation: capture-reveal 140ms cubic-bezier(.16, 1, .3, 1); }
  .ready .capture-tools, .ready .capture-actions, .unavailable .capture-tools, .unavailable .capture-actions { opacity: 1; pointer-events: auto; }
  .ready .capture-actions { animation-name: capture-controls-in; }
  @keyframes capture-reveal { from { opacity: 0; } to { opacity: 1; } }
  @keyframes capture-controls-in { from { opacity: 0; translate: 0 -4px; } to { opacity: 1; translate: 0 0; } }
  @media (prefers-reduced-motion: reduce) { .ready .capture-dim, .ready .capture-tools, .ready .capture-actions { animation: none; } }
  .selection-surface { position: absolute; inset: 0; cursor: crosshair; touch-action: none; }
  .selection { position: fixed; pointer-events: none; border: 1px solid #a3e9d1; box-shadow: 0 0 0 100vmax rgb(0 0 0 / .45); }
  .finished { z-index: 1; }
  .capture-tools { position: relative; z-index: 10; }
  .capture-editor .capture-tools.busy { pointer-events: none; }
  .capture-actions { position: fixed; z-index: 20; top: 18px; left: 50%; transform: translateX(-50%); max-width: calc(100vw - 24px); }
  /* Keep the shared control independent of the host page's button/kbd defaults. */
  .capture-actions button { display: inline-flex; flex-shrink: 0; align-items: center; justify-content: center; gap: 7px; min-height: 34px; border-radius: 8px; font: 600 12px/normal -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 0 10px; appearance: none; border: 0; box-shadow: none; cursor: pointer; -webkit-font-smoothing: antialiased; -webkit-tap-highlight-color: transparent; }
  .capture-actions button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .capture-actions button:disabled { opacity: .45; cursor: default; }
  .copy { background: #a3e9d1; color: #143d33; }
  .copy:hover:not(:disabled) { background: #b9f0dc; }
  .copy kbd { font: inherit; font-size: 11px; padding: 3px 5px; border-radius: 4px; background: #143d331a; color: inherit; }
  .capture-error { position: fixed; z-index: 25; top: 86px; left: 50%; transform: translateX(-50%); max-width: calc(100vw - 32px); border: 1px solid var(--border); border-radius: 10px; background: var(--surface); color: var(--error-text); padding: 12px 16px; font-size: 13px; }
  @media (max-width: 760px) { .copy kbd { display: none; } .capture-actions button { padding: 0 8px; } }
</style>
