<script lang="ts">
  import { onMount } from 'svelte';
  import Overlay from '@glassboard/ui/Overlay.svelte';
  import Toolbar from '@glassboard/ui/Toolbar.svelte';
  import { action, defaults, savePreferences, shortcutLabel, subscribe, type Session } from '@glassboard/ui/session';
  import { drawingKeydown } from '@glassboard/ui/keys';
  import '@glassboard/ui/toolbar.css';

  // The real overlay and toolbar, driven by the same browser adapter the desktop
  // app uses for its preview. Anything with data-glassboard-try starts a session;
  // the banner button (or Escape) ends it. The app's global shortcut is not
  // mirrored here, so page keyboard shortcuts stay predictable.
  let session = $state<Session>(structuredClone(defaults));
  let error = $state('');
  let errorTimer: ReturnType<typeof setTimeout>;
  function onerror(e: unknown) { error = String(e); if (e) console.error(e); clearTimeout(errorTimer); errorTimer = setTimeout(() => error = '', 8000); }
  const run = (name: string) => action(name).catch(onerror);
  function keydown(event: KeyboardEvent) {
    drawingKeydown(event, session, { run, save: preferences => savePreferences(preferences).catch(onerror), toggleShortcut: false });
  }
  function click(event: MouseEvent) {
    if (!(event.target as Element | null)?.closest('[data-glassboard-try]')) return;
    event.preventDefault();
    (event.target as HTMLElement).blur();
    run('show');
  }
  let drawing = $derived(session.mode === 'draw');
  $effect(() => { document.documentElement.classList.toggle('annotating', drawing); });
  onMount(() => {
    let disposed = false, stop = () => {};
    subscribe(value => session = value).then(fn => { if (disposed) fn(); else stop = fn; }).catch(onerror);
    return () => { disposed = true; stop(); clearTimeout(errorTimer); document.documentElement.classList.remove('annotating'); };
  });
</script>

<svelte:window onkeydown={keydown} onclick={click} />
<div class="glassboard-layer">
  <Overlay {session} {onerror} />
  <Toolbar {session} {error} {onerror} />
  {#if drawing}
    <div class="banner" role="status">
      <p>This is the real Glassboard toolbar. Pick a tool and draw. <span class="undo"><kbd>{shortcutLabel('CommandOrControl+Z')}</kbd> undoes.</span></p>
      <button type="button" onclick={() => run('hide')}>Back to the page <kbd>Esc</kbd></button>
    </div>
  {/if}
</div>

<style>
  /* One stacking context above all page content, so the canvas always wins. */
  .glassboard-layer { position: relative; z-index: 50; }
  .banner { position: fixed; z-index: 20; top: 16px; left: 50%; transform: translateX(-50%); display: flex; align-items: center; gap: 16px; max-width: calc(100vw - 32px); padding: 8px 8px 8px 18px; border-radius: 999px; background: #143d33; color: #e6f1ec; font-family: inherit; font-size: 14px; line-height: 1.4; box-shadow: 0 8px 24px #0a1f1a55; animation: banner-in 180ms ease-out both; }
  .banner p { margin: 0; }
  .banner kbd { font: inherit; font-size: 12px; padding: 1px 6px; border-radius: 5px; background: #ffffff1f; color: inherit; }
  .banner button { display: inline-flex; align-items: center; gap: 8px; flex-shrink: 0; padding: 8px 14px; border: 0; border-radius: 999px; background: #a3e9d1; color: #143d33; font: inherit; font-weight: 600; cursor: pointer; }
  .banner button kbd { background: #143d3320; }
  .banner button:hover { background: #b9f0dc; }
  .banner button:focus-visible { outline: 2px solid #a3e9d1; outline-offset: 2px; }
  @keyframes banner-in { from { opacity: 0; transform: translate(-50%, -6px); } to { opacity: 1; transform: translate(-50%, 0); } }
  @media (prefers-reduced-motion: reduce) { .banner { animation: none; } }
  @media (max-width: 640px) { .banner { top: 12px; flex-direction: column; align-items: stretch; gap: 8px; padding: 12px 16px; border-radius: 18px; text-align: center; } .banner .undo { display: none; } }
</style>
