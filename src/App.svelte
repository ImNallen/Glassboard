<script lang="ts">
  import { onMount } from 'svelte';
  import Overlay from './Overlay.svelte';
  import Toolbar from './Toolbar.svelte';
  import Settings from './Settings.svelte';
  import { action, defaults, native, savePreferences, shortcutLabel, subscribe, type Session } from './lib/session';
  import { eraseShortcut, toolShortcut } from './lib/shortcuts';
  import { protectSelection } from './lib/selection';
  const surface = new URLSearchParams(location.search).get('surface');
  let session = $state<Session>(structuredClone(defaults));
  let error = $state('');
  let errorTimer: ReturnType<typeof setTimeout>;
  function onerror(e: unknown) { error = String(e); console.error(e); clearTimeout(errorTimer); errorTimer = setTimeout(() => error = '', 8000); }
  function run(name: string) { action(name).catch(onerror); }
  function keydown(event: KeyboardEvent) {
    if (surface === 'settings') {
      if (event.key === 'Escape') { event.preventDefault(); run('close-settings'); }
      return;
    }
    if ((event.target as HTMLElement)?.closest('input, textarea, select, [contenteditable="true"]')) return;
    const key = event.key.toLowerCase();
    const command = event.metaKey || event.ctrlKey;
    // Browser preview mirrors the default show/hide binding; the native binding lives in Rust.
    if (!native && command && event.shiftKey && key === 'a') { event.preventDefault(); run('toggle'); return; }
    if (session.mode === 'hidden') return;
    if (session.mode === 'draw' && eraseShortcut(event)) { event.preventDefault(); run('erase'); return; }
    if (key === 'escape') { event.preventDefault(); run('hide'); return; }
    if (command && key === 'z') { event.preventDefault(); run(event.shiftKey ? 'redo' : 'undo'); return; }
    if (!command && !event.altKey && !event.shiftKey && key === 'v') { event.preventDefault(); run('interact'); return; }
    const tool = toolShortcut(event);
    if (tool) {
      event.preventDefault(); savePreferences({ ...session.preferences, tool }).catch(onerror);
      if (session.mode !== 'draw') run('interact');
    }
  }
  onMount(() => {
    const stopSelectionGuard = protectSelection();
    let disposed = false, stop = () => {};
    subscribe(value => session = value).then(fn => { if (disposed) fn(); else stop = fn; }).catch(onerror);
    return () => { disposed = true; stop(); stopSelectionGuard(); clearTimeout(errorTimer); };
  });
</script>
<svelte:window onkeydown={keydown}/>
{#if !native && surface !== 'settings'}
  <main class="preview-background">
    <div class="preview-top"><span class="preview-logo">↗ Glassboard</span><span class="preview-badge">BROWSER PREVIEW</span></div>
    <div class="preview-copy"><span class="eyebrow">A LITTLE CLARITY GOES A LONG WAY</span><h1>Your screen.<br/>Your point.</h1><p>Draw attention to what matters.<br/>Pick a tool below and make your mark.</p><div class="preview-shortcuts"><kbd>{shortcutLabel(session.preferences.shortcut)}</kbd><span>Show / hide</span><kbd>Shift</kbd><span>Constrain shapes</span></div></div>
    <div class="preview-footer"><span>Arrows. Shapes. A little emphasis.</span><span>Nothing between you and your point.</span></div>
  </main>
  {#if session.mode === 'hidden'}<button class="preview-restore" onclick={() => run('toggle')}>Show Glassboard</button>{/if}
{/if}
{#if surface === 'overlay' || (!native && surface !== 'settings')}<Overlay {session} {onerror}/>{/if}
{#if surface === 'toolbar' || (!native && surface !== 'settings')}<Toolbar {session} {error} {onerror}/>{/if}

{#if surface === 'settings'}<Settings {session} {error} {onerror}/>{/if}
