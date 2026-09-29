<script lang="ts">
  import { onMount } from 'svelte';
  import Overlay from '@glassboard/ui/Overlay.svelte';
  import Toolbar from '@glassboard/ui/Toolbar.svelte';
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
  // A non-interactive copy of the toolbar shows where the real one will appear.
  // It is replaced by the live toolbar while drawing.
  const previewSession: Session = { ...structuredClone(defaults), mode: 'draw' };
  $effect(() => { document.documentElement.classList.toggle('annotating', drawing); });
  onMount(() => {
    let disposed = false, stop = () => {};
    subscribe(value => session = value).then(fn => { if (disposed) fn(); else stop = fn; }).catch(onerror);
    return () => { disposed = true; stop(); clearTimeout(errorTimer); document.documentElement.classList.remove('annotating'); };
  });
</script>

<svelte:window onkeydown={keydown} onclick={click} />
<div class="glassboard-layer">
  {#if !drawing}
    <div class="toolbar-preview" inert aria-hidden="true"><Toolbar session={previewSession} error="" onerror={() => {}} pinned /></div>
  {/if}
  <Overlay {session} {onerror} />
  <!-- Pinned: on a web page there is no screen edge to tuck into, so the toolbar stays open. -->
  <Toolbar {session} {error} {onerror} pinned />
  {#if drawing}
    <div class="banner" role="status">
      <p>This is the real toolbar. Pick a tool and draw over the page.</p>
      <button type="button" onclick={() => run('hide')}>Done <kbd>Esc</kbd></button>
    </div>
  {/if}
</div>

<style>
  /* One stacking context above all page content, so the canvas always wins. */
  .glassboard-layer { position: relative; z-index: 50; }
  .toolbar-preview :global(.toolbar-host) { pointer-events: none; }
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
