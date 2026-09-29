<script lang="ts">
  import { onMount } from 'svelte';
  import Overlay from '@glassboard/ui/Overlay.svelte';
  import Toolbar from '@glassboard/ui/Toolbar.svelte';
  import Settings from './Settings.svelte';
  import Tutorial from './Tutorial.svelte';
  import Capture from './Capture.svelte';
  import { action, defaults, native, savePreferences, shortcutLabel, subscribe, type Session } from '@glassboard/ui/session';
  import { drawingKeydown } from '@glassboard/ui/keys';
  import { protectSelection } from '@glassboard/ui/selection';
  const surface = new URLSearchParams(location.search).get('surface');
  let session = $state<Session>(structuredClone(defaults));
  let error = $state('');
  let errorTimer: ReturnType<typeof setTimeout>;
  function onerror(e: unknown) { error = String(e); console.error(e); clearTimeout(errorTimer); errorTimer = setTimeout(() => error = '', 8000); }
  function run(name: string) { action(name).catch(onerror); }
  function keydown(event: KeyboardEvent) {
    if (surface === 'capture' || session.capture) return;
    if (surface === 'tutorial' && event.key === 'Escape') { event.preventDefault(); run(session.mode === 'draw' ? 'hide' : 'dismiss-tutorial'); return; }
    if (surface === 'settings') {
      if (event.key === 'Escape') { event.preventDefault(); run('close-settings'); }
      return;
    }
    drawingKeydown(event, session, { run, save: preferences => savePreferences(preferences).catch(onerror) });
  }
  onMount(() => {
    const stopSelectionGuard = protectSelection();
    let disposed = false, stop = () => {};
    subscribe(value => session = value).then(fn => { if (disposed) fn(); else stop = fn; }).catch(onerror);
    return () => { disposed = true; stop(); stopSelectionGuard(); clearTimeout(errorTimer); };
  });
</script>
<svelte:window onkeydown={keydown}/>
{#if !native && surface !== 'settings' && surface !== 'capture' && !session.capture}
  <main class="preview-background">
    <div class="preview-top"><span class="preview-logo">↗ Glassboard</span><span class="preview-badge">BROWSER PREVIEW</span></div>
    <div class="preview-copy"><span class="eyebrow">A LITTLE CLARITY GOES A LONG WAY</span><h1>Your screen.<br/>Your point.</h1><p>Draw attention to what matters.<br/>Pick a tool below and make your mark.</p><div class="preview-shortcuts"><kbd>{shortcutLabel(session.preferences.shortcut)}</kbd><span>Annotate / work</span><kbd>Shift</kbd><span>Constrain shapes</span></div></div>
    <div class="preview-footer"><span>Arrows. Shapes. A little emphasis.</span><span>Nothing between you and your point.</span></div>
  </main>
  {#if session.mode === 'hidden'}<div class="preview-actions"><button class="preview-restore" onclick={() => run('toggle')}>Start annotating</button></div>{/if}
{/if}
{#if !session.capture && (surface === 'overlay' || (!native && surface !== 'settings' && surface !== 'capture'))}<Overlay {session} {onerror}/>{/if}
{#if !session.capture && (surface === 'toolbar' || (!native && surface !== 'settings' && surface !== 'capture'))}<Toolbar {session} {error} {onerror} oncapture={() => run('capture')}/>{/if}

{#if surface === 'settings'}<Settings {session} {error} {onerror}/>{/if}

{#if !session.capture && (surface === 'tutorial' || (!native && surface !== 'settings' && surface !== 'capture'))}<Tutorial {session} {error} {onerror}/>{/if}

{#if session.capture?.ready && (surface === 'capture' || !native)}{#key session.capture.id}<Capture {session} {onerror}/>{/key}{/if}
