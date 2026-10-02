<script lang="ts">
  import { onMount } from 'svelte';
  import Overlay from '@glassboard/ui/Overlay.svelte';
  import Toolbar from '@glassboard/ui/Toolbar.svelte';
  import Settings from './Settings.svelte';
  import Tutorial from './Tutorial.svelte';
  import PreviewBackdrop from './PreviewBackdrop.svelte';
  import Capture from '@glassboard/ui/Capture.svelte';
  import { previewCapture } from '@glassboard/ui/capture-preview';
  import { action, defaults, native, savePreferences, subscribe, type Session, type Action } from '@glassboard/ui/session';
  import { nativeCapture } from './lib/capture';
  import { parseSurface, windowParts } from './lib/surface';
  import { drawingKeydown } from '@glassboard/ui/keys';
  import { createErrors } from '@glassboard/ui/errors';
  import { protectSelection } from '@glassboard/ui/selection';
  const surface = parseSurface(location.search);
  let session = $state<Session>(structuredClone(defaults));
  let parts = $derived(windowParts(surface, native, session.capture));
  const errors = createErrors(() => session);
  const capture = native ? nativeCapture : previewCapture;
  function run(name: Action) { action(name).catch(errors.report); }
  function keydown(event: KeyboardEvent) {
    if (surface === 'capture' || session.capture) return;
    if (surface === 'tutorial' && event.key === 'Escape') { event.preventDefault(); run(session.mode === 'draw' ? 'hide' : 'dismiss-tutorial'); return; }
    if (surface === 'settings') {
      if (event.key === 'Escape') { event.preventDefault(); run('close-settings'); }
      return;
    }
    drawingKeydown(event, session, { run, save: preferences => savePreferences(preferences).catch(errors.report), capture: () => run('capture') });
  }
  onMount(() => {
    const stopSelectionGuard = protectSelection();
    let disposed = false, stop = () => {};
    subscribe(value => session = value).then(fn => { if (disposed) fn(); else stop = fn; }).catch(errors.report);
    return () => { disposed = true; stop(); stopSelectionGuard(); errors.clear(); };
  });
</script>
<svelte:window onkeydown={keydown}/>
{#if parts.includes('backdrop')}<PreviewBackdrop {session} {run}/>{/if}
{#if parts.includes('overlay')}<Overlay {session} onerror={errors.report}/>{/if}
{#if parts.includes('toolbar')}<Toolbar {session} {errors} host={native ? 'window' : 'page'} oncapture={() => run('capture')}/>{/if}
{#if parts.includes('settings')}<Settings {session} {errors}/>{/if}
{#if parts.includes('tutorial')}<Tutorial {session} {errors}/>{/if}
{#if parts.includes('capture')}{#key session.capture?.id}<Capture {session} {...capture}/>{/key}{/if}
