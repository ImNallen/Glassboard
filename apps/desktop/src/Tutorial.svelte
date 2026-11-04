<script lang="ts">
  import { ArrowUpRight, Check, X } from '@lucide/svelte';
  import Logo from '@glassboard/ui/Logo.svelte';
  import { action, native, type Session, type Action } from '@glassboard/ui/session';
  import { keyLabel, shortcutLabel } from '@glassboard/ui/shortcuts';
  import type { Errors } from '@glassboard/ui/errors';
  import { t } from '@glassboard/ui/i18n';
  import Rich from './Rich.svelte';
  let { session, errors }: { session: Session; errors: Errors } = $props();
  let busy = $state(false);
  let shortcut = $derived(shortcutLabel(session.preferences.shortcut));
  let undo = $derived(keyLabel(session.preferences.keybindings, 'undo'));
  let hide = $derived(keyLabel(session.preferences.keybindings, 'hide'));
  async function run(name: Action) {
    busy = true;
    try { await action(name); } catch (e) { errors.report(e); }
    finally { busy = false; }
  }
</script>

{#snippet kbd(text: string)}<kbd>{text}</kbd>{/snippet}

{#if session.tutorial}
  <aside class="tutorial" class:preview={!native} aria-label={t('Glassboard tutorial')}>
    <header>
      <span class="brand"><Logo size={20}/> Glassboard</span>
      <button class="icon-button" aria-label={t('Dismiss tutorial')} title={t('Dismiss tutorial')} disabled={busy} onclick={() => run('dismiss-tutorial')}><X size={16}/></button>
    </header>
    <div aria-live="polite" aria-atomic="true">
      {#if session.tutorial === 'welcome'}
        <span class="step">{t('A QUICK INTRODUCTION')}</span>
        <h1>{t('Your point, made clear.')}</h1>
        <p><Rich message={'Press <kbd>{shortcut}</kbd> to draw over your screen. Press it again to get back to work.'} args={{ shortcut }} {kbd}/></p>
        <p class="hint">{t('Returning to work clears your marks. Your next annotation session starts fresh.')}</p>
      {:else if session.tutorial === 'draw'}
        <span class="step">{t('1 OF 2 · MAKE A MARK')}</span>
        <h1>{t('Point something out.')}</h1>
        <p>{t('Drag anywhere outside this guide to draw. Choose a pen, arrow, shape, or text from the open toolbar.')}</p>
        <p class="hint">{t('Hold Shift for straight arrows or equal-sided shapes.')}{#if undo}{' '}<Rich message={'Undo with <kbd>{shortcut}</kbd>.'} args={{ shortcut: undo }} {kbd}/>{/if}</p>
      {:else if session.tutorial === 'hide'}
        <span class="step">{t('2 OF 2 · BACK TO WORK')}</span>
        <h1>{t('Made your point?')}</h1>
        <p><Rich message={'Press <kbd>{shortcut}</kbd> to hide Glassboard and use your screen again.'} args={{ shortcut }} {kbd}/></p>
        <p class="hint">{#if hide}<Rich message={'<kbd>{shortcut}</kbd> also returns you to work.'} args={{ shortcut: hide }} {kbd}/>{' '}{/if}{t('Your next annotation session starts clear.')}</p>
      {:else}
        <span class="step"><Check size={13}/> {t('YOU’RE READY')}</span>
        <h1>{t('Work. Annotate. Repeat.')}</h1>
        <p><Rich message={'Press <kbd>{shortcut}</kbd> whenever you want to make another point. You’ll start with a clear screen.'} args={{ shortcut }} {kbd}/></p>
        <p class="hint">{t('Find settings and this tutorial in the Glassboard menu-bar or tray icon.')}</p>
      {/if}
    </div>
    <footer>
      {#if session.tutorial === 'welcome'}
        <button class="secondary" disabled={busy} onclick={() => run('dismiss-tutorial')}>{t('Skip tutorial')}</button>
        <button class="primary" disabled={busy} onclick={() => run('tutorial-start')}>{t('Try annotating')} <ArrowUpRight size={15}/></button>
      {:else if session.tutorial === 'done'}
        <button class="primary" disabled={busy} onclick={() => run('dismiss-tutorial')}>{t('Got it')} <Check size={15}/></button>
      {:else}
        <button class="secondary" disabled={busy} onclick={() => run('dismiss-tutorial')}>{t('Skip tutorial')}</button>
        <span class="mode">{t('Annotation mode')}</span>
      {/if}
    </footer>
    {#if session.unavailableShortcuts.includes('toggle')}
      <p class="error" role="alert">
        <Rich message={'Another app is using <kbd>{shortcut}</kbd>. <button>Choose a different shortcut</button>'} args={{ shortcut }} {kbd}>
          {#snippet button(label: string)}<button class="link" disabled={busy} onclick={() => run('settings')}>{label}</button>{/snippet}
        </Rich>
      </p>
    {/if}
    {#if errors.message}
      <p class="error" role="alert">{errors.message} <button class="link" onclick={errors.dismiss}>{t('Dismiss')}</button></p>
    {/if}
  </aside>
{/if}

<style>
  .tutorial { color-scheme: light dark; margin: 8px; padding: 20px; height: calc(100vh - 16px); overflow-y: auto; background: var(--gb-surface); border: 1px solid var(--gb-border); border-radius: 16px; box-shadow: 0 3px 10px var(--gb-shadow); display: flex; flex-direction: column; }
  .tutorial.preview { position: fixed; z-index: 30; top: 20px; left: 50%; transform: translateX(-50%); width: min(384px, calc(100vw - 32px)); height: auto; min-height: 300px; max-height: calc(100vh - 120px); margin: 0; }
  header, footer { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  header { margin-bottom: 18px; }
  header .icon-button { width: 26px; height: 26px; margin: -6px; }
  .brand { display: flex; align-items: center; gap: 5px; font-size: 12px; font-weight: 600; }
  .step { display: flex; align-items: center; gap: 5px; font-size: 9px; letter-spacing: 1.2px; color: var(--gb-muted); font-weight: 600; }
  h1 { font-size: 24px; letter-spacing: -.8px; line-height: 1.2; margin: 9px 0 12px; font-weight: 600; }
  p { font-size: 13px; line-height: 1.6; margin: 0 0 12px; }
  kbd { white-space: nowrap; }
  .hint { color: var(--gb-muted); font-size: 12px; }
  footer { margin-top: auto; padding-top: 6px; }
  footer button { font-size: 12px; min-height: 34px; border-radius: 8px; }
  .primary { display: flex; align-items: center; gap: 8px; background: var(--gb-selected); padding: 8px 12px; margin-left: auto; transition: background .12s, transform 160ms var(--gb-ease-out); }
  .primary:hover { background: var(--gb-selected-border); }
  @media (prefers-reduced-motion: no-preference) { .primary:active:not(:disabled) { transform: scale(0.97); } }
  .secondary { color: var(--gb-secondary-text); padding: 0; }
  .secondary:hover { text-decoration: underline; }
  .mode { font-size: 11px; color: var(--gb-muted); }
  .error { margin: 8px 0 0; color: var(--gb-error-text); font-size: 11px; overflow-wrap: anywhere; }
  .link { padding: 0; font-size: inherit; color: inherit; text-decoration: underline; }
</style>
