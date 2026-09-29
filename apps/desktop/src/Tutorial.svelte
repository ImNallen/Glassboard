<script lang="ts">
  import { ArrowUpRight, Check, X } from '@lucide/svelte';
  import Logo from '@glassboard/ui/Logo.svelte';
  import { action, native, shortcutLabel, type Session } from '@glassboard/ui/session';
  let { session, error, onerror }: { session: Session; error: string; onerror: (error: unknown) => void } = $props();
  let busy = $state(false);
  let shortcut = $derived(shortcutLabel(session.preferences.shortcut));
  async function run(name: string) {
    busy = true;
    try { await action(name); } catch (e) { onerror(e); }
    finally { busy = false; }
  }
</script>

{#if session.tutorial}
  <aside class="tutorial" class:preview={!native} aria-label="Glassboard tutorial">
    <header>
      <span class="brand"><Logo size={20}/> Glassboard</span>
      <button class="icon-button" aria-label="Dismiss tutorial" title="Dismiss tutorial" disabled={busy} onclick={() => run('dismiss-tutorial')}><X size={16}/></button>
    </header>
    <div aria-live="polite" aria-atomic="true">
      {#if session.tutorial === 'welcome'}
        <span class="step">A QUICK INTRODUCTION</span>
        <h1>Your point, made clear.</h1>
        <p>Press <kbd>{shortcut}</kbd> to draw over your screen. Press it again to get back to work.</p>
        <p class="hint">Returning to work clears your marks. Your next annotation session starts fresh.</p>
      {:else if session.tutorial === 'draw'}
        <span class="step">1 OF 2 · MAKE A MARK</span>
        <h1>Point something out.</h1>
        <p>Drag anywhere outside this guide to draw. Choose a pen, arrow, shape, or text from the open toolbar.</p>
        <p class="hint">Hold Shift for straight arrows or equal-sided shapes. Undo with <kbd>{shortcutLabel('CommandOrControl+Z')}</kbd>.</p>
      {:else if session.tutorial === 'hide'}
        <span class="step">2 OF 2 · BACK TO WORK</span>
        <h1>Made your point?</h1>
        <p>Press <kbd>{shortcut}</kbd> to hide Glassboard and use your screen again.</p>
        <p class="hint">Escape also returns you to work. Your next annotation session starts clear.</p>
      {:else}
        <span class="step"><Check size={13}/> YOU’RE READY</span>
        <h1>Work. Annotate. Repeat.</h1>
        <p>Press <kbd>{shortcut}</kbd> whenever you want to make another point. You’ll start with a clear screen.</p>
        <p class="hint">Find settings and this tutorial in the Glassboard menu-bar or tray icon.</p>
      {/if}
    </div>
    <footer>
      {#if session.tutorial === 'welcome'}
        <button class="secondary" disabled={busy} onclick={() => run('dismiss-tutorial')}>Skip tutorial</button>
        <button class="primary" disabled={busy} onclick={() => run('tutorial-start')}>Try annotating <ArrowUpRight size={15}/></button>
      {:else if session.tutorial === 'done'}
        <button class="primary" disabled={busy} onclick={() => run('dismiss-tutorial')}>Got it <Check size={15}/></button>
      {:else}
        <button class="secondary" disabled={busy} onclick={() => run('dismiss-tutorial')}>Skip tutorial</button>
        <span class="mode">Annotation mode</span>
      {/if}
    </footer>
    {#if error || session.error}<p class="error" role="alert">{error || session.error}</p>{/if}
  </aside>
{/if}

<style>
  .tutorial { color-scheme: light dark; margin: 8px; padding: 20px; height: calc(100vh - 16px); overflow-y: auto; background: var(--surface); border: 1px solid var(--border); border-radius: 16px; box-shadow: 0 3px 10px var(--shadow); display: flex; flex-direction: column; }
  .tutorial.preview { position: fixed; z-index: 30; top: 20px; left: 50%; transform: translateX(-50%); width: min(384px, calc(100vw - 32px)); height: auto; min-height: 300px; max-height: calc(100vh - 120px); margin: 0; }
  header, footer { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  header { margin-bottom: 18px; }
  header .icon-button { width: 26px; height: 26px; margin: -6px; }
  .brand { display: flex; align-items: center; gap: 5px; font-size: 12px; font-weight: 600; }
  .step { display: flex; align-items: center; gap: 5px; font-size: 9px; letter-spacing: 1.2px; color: var(--muted); font-weight: 600; }
  h1 { font-size: 24px; letter-spacing: -.8px; line-height: 1.2; margin: 9px 0 12px; font-weight: 600; }
  p { font-size: 13px; line-height: 1.6; margin: 0 0 12px; }
  kbd { white-space: nowrap; }
  .hint { color: var(--muted); font-size: 12px; }
  footer { margin-top: auto; padding-top: 6px; }
  footer button { font-size: 12px; min-height: 34px; border-radius: 8px; }
  .primary { display: flex; align-items: center; gap: 8px; background: var(--selected); padding: 8px 12px; margin-left: auto; }
  .primary:hover { background: var(--selected-border); }
  .secondary { color: var(--secondary-text); padding: 0; }
  .secondary:hover { text-decoration: underline; }
  .mode { font-size: 11px; color: var(--muted); }
  .error { margin: 8px 0 0; color: var(--error-text); font-size: 11px; overflow-wrap: anywhere; }
</style>
