<script lang="ts">
  import { onMount } from 'svelte';
  import { action, defaults, native, savePreferences, subscribe, type Session, type Action } from '@glassboard/ui/session';
  import { parseSurface, windowParts } from './lib/surface';
  import { drawingKeydown } from '@glassboard/ui/keys';
  import { createErrors } from '@glassboard/ui/errors';
  import { protectSelection } from '@glassboard/ui/selection';
  const surface = parseSurface(location.search);
  let session = $state<Session>(structuredClone(defaults));
  let parts = $derived(windowParts(surface, native, session.capture));
  const errors = createErrors(() => session);
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
  async function loadCapture() {
    const { default: Capture } = await import('@glassboard/ui/Capture.svelte');
    if (native) {
      const { nativeCapture } = await import('./lib/capture');
      return { Capture, props: nativeCapture };
    }
    const { previewCapture } = await import('@glassboard/ui/capture-preview');
    return { Capture, props: previewCapture };
  }
  let captureLoader: ReturnType<typeof loadCapture> | undefined;
  function loadCaptureOnce() {
    return (captureLoader ??= loadCapture());
  }
  onMount(() => {
    const stopSelectionGuard = protectSelection();
    let disposed = false, stop = () => {};
    subscribe(value => session = value).then(fn => { if (disposed) fn(); else stop = fn; }).catch(errors.report);
    return () => { disposed = true; stop(); stopSelectionGuard(); errors.clear(); };
  });
</script>
<svelte:window onkeydown={keydown}/>
{#if parts.includes('backdrop')}
  {#await import('./PreviewBackdrop.svelte') then { default: PreviewBackdrop }}
    <PreviewBackdrop {session} {run}/>
  {/await}
{/if}
{#if parts.includes('overlay')}
  {#await import('@glassboard/ui/Overlay.svelte') then { default: Overlay }}
    <Overlay {session} onerror={errors.report}/>
  {/await}
{/if}
{#if parts.includes('toolbar')}
  {#await import('@glassboard/ui/Toolbar.svelte') then { default: Toolbar }}
    <Toolbar {session} {errors} host={native ? 'window' : 'page'} oncapture={() => run('capture')}/>
  {/await}
{/if}
{#if parts.includes('settings')}
  {#await import('./Settings.svelte') then { default: Settings }}
    <Settings {session} {errors}/>
  {/await}
{/if}
{#if parts.includes('tutorial')}
  {#await import('./Tutorial.svelte') then { default: Tutorial }}
    <Tutorial {session} {errors}/>
  {/await}
{/if}
{#if parts.includes('capture')}
  {#await loadCaptureOnce() then { Capture, props }}
    {#key session.capture?.id}<Capture {session} {...props}/>{/key}
  {/await}
{/if}
