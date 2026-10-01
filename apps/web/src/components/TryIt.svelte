<script lang="ts">
  import { onMount } from 'svelte';
  import Overlay from '@glassboard/ui/Overlay.svelte';
  import Toolbar from '@glassboard/ui/Toolbar.svelte';
  import Capture from '@glassboard/ui/Capture.svelte';
  import { copyPreviewCapture } from '@glassboard/ui/capture-preview';
  import { action, defaults, savePreferences, subscribe, type Session } from '@glassboard/ui/session';
  import { drawingKeydown } from '@glassboard/ui/keys';
  import '@glassboard/ui/toolbar.css';

  // The real overlay and toolbar, driven by the same browser adapter the desktop
  // app uses for its preview. Anything with data-glassboard-try starts a session;
  // the banner button (or Escape) ends it. The app's global shortcut is not
  // mirrored here, so page keyboard shortcuts stay predictable.
  let session = $state<Session>(structuredClone(defaults));
  let error = $state('');
  let errorTimer: ReturnType<typeof setTimeout>;
  let frame: HTMLCanvasElement | null = null;
  let preparingCapture = $state(false);
  let captureRequest = 0;
  let disposed = false;
  let enabled = $state(false);
  function onerror(e: unknown) { error = String(e); if (e) console.error(e); clearTimeout(errorTimer); errorTimer = setTimeout(() => error = '', 8000); }
  const run = (name: string) => action(name).catch(onerror);
  async function startCapture() {
    if (!enabled || preparingCapture || session.capture || session.mode !== 'draw') return;
    const request = ++captureRequest;
    preparingCapture = true;
    try {
      const { capturePage } = await import('../lib/capture');
      const image = await capturePage();
      if (disposed || request !== captureRequest || session.mode !== 'draw') return;
      frame = image;
      await action('capture');
    } catch (cause) { if (!disposed && request === captureRequest) onerror(cause); }
    finally { if (request === captureRequest) preparingCapture = false; }
  }
  async function getImage() {
    if (!frame) throw new Error('The screenshot is unavailable. Cancel and try again.');
    return frame;
  }
  function keydown(event: KeyboardEvent) {
    if (!enabled || session.capture) return;
    drawingKeydown(event, session, { run, save: preferences => savePreferences(preferences).catch(onerror), capture: startCapture, toggleShortcut: false });
  }
  function click(event: MouseEvent) {
    if (!enabled || !(event.target as Element | null)?.closest('[data-glassboard-try]')) return;
    event.preventDefault();
    (event.target as HTMLElement).blur();
    run('show');
  }
  let drawing = $derived(session.mode === 'draw');
  let capturing = $derived(Boolean(session.capture));
  const copyImage = (_id: number, image: Promise<Blob>) => copyPreviewCapture(image);
  $effect(() => { document.documentElement.classList.toggle('annotating', enabled && (drawing || capturing)); });
  onMount(() => {
    // Match the page's demo button breakpoint and end any active session on phones.
    const desktop = matchMedia('(min-width: 761px)');
    function updateAvailability() {
      enabled = desktop.matches;
      if (!enabled) run('hide');
    }
    updateAvailability();
    desktop.addEventListener('change', updateAvailability);
    let stop = () => {};
    subscribe(value => {
      session = value;
      if (value.mode === 'hidden' && !value.capture) { frame = null; preparingCapture = false; captureRequest++; }
    }).then(fn => { if (disposed) fn(); else stop = fn; }).catch(onerror);
    return () => { disposed = true; frame = null; stop(); desktop.removeEventListener('change', updateAvailability); clearTimeout(errorTimer); document.documentElement.classList.remove('annotating'); };
  });
</script>

<svelte:window onkeydown={keydown} onclick={click} />
{#if enabled && (drawing || capturing)}
  <div class="glassboard-layer">
    {#if !capturing}
      <Overlay {session} {onerror} />
      <!-- Pinned: on a web page there is no screen edge to tuck into, so the toolbar stays open. -->
      <Toolbar {session} {error} {onerror} oncapture={startCapture} pinned />
    {:else if session.capture?.ready}
      {#key session.capture.id}<Capture {session} {onerror} {getImage} {copyImage} />{/key}
    {/if}
    <div class="banner" role="status">
      <p>{preparingCapture ? 'Preparing screenshot…' : capturing ? 'Drag to select part of this page. Annotate it, then copy and paste it anywhere.' : 'Pick a tool and draw over the page.'}</p>
      <button type="button" onclick={() => run(capturing ? 'cancel-capture' : 'hide')}>Done <kbd>Esc</kbd></button>
    </div>
  </div>
{/if}

<style>
  /* One stacking context above all page content, so the canvas always wins. */
  .glassboard-layer { position: relative; z-index: 50; }
  /* Styled as a sibling of the toolbar (same surface variables), placed below the toolbar's
     stacking layer so tooltips can rise over it. */
  .banner { position: fixed; z-index: 5; bottom: 124px; left: 50%; transform: translateX(-50%); display: flex; align-items: center; gap: 14px; width: max-content; max-width: calc(100vw - 24px); padding: 8px 8px 8px 16px; border: 1px solid var(--border); border-radius: 14px; background: var(--surface); color: var(--text); font-size: 14px; line-height: 1.4; box-shadow: 0 4px 10px var(--shadow); color-scheme: light dark; animation: banner-in 180ms ease-out both; }
  .banner p { margin: 0; min-width: 0; }
  .banner button { display: inline-flex; align-items: center; gap: 8px; flex-shrink: 0; min-height: 34px; padding: 0 12px 0 14px; border: 0; border-radius: 9px; background: #a3e9d1; color: #143d33; font: inherit; font-weight: 600; cursor: pointer; }
  .banner button:hover { background: #b9f0dc; }
  .banner button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .banner kbd { font: inherit; font-size: 11px; line-height: 1; padding: 3px 5px; border-radius: 4px; background: #143d331f; }
  @keyframes banner-in { from { opacity: 0; } to { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .banner { animation: none; } }
  /* No keyboard to speak of: drop the Esc hint and sit closer to the toolbar. */
  @media (hover: none) { .banner { bottom: 76px; } .banner kbd { display: none; } }
  @media (max-width: 640px) { .banner { left: 12px; right: 12px; width: auto; max-width: none; transform: none; padding: 8px 8px 8px 14px; font-size: 13px; } .banner button { padding: 0 12px; } .banner kbd { display: none; } }
</style>
