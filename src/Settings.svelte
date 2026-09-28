<script lang="ts">
  import { untrack } from 'svelte';
  import { X, Trash2, Power } from '@lucide/svelte';
  import { action, native, savePreferences, shortcutLabel, type ToolbarPosition, type Session } from './lib/session';
  let { session, error, onerror }: { session: Session; error: string; onerror: (error: unknown) => void } = $props();
  let shortcut = $state('');
  let saving = $state(false);
  let saved = $state(false);
  let placing = $state(false);
  let windowVisible = $derived(session.settingsOpen || !native);
  const run = (name: string) => action(name, true).catch(onerror);
  $effect(() => {
    if (windowVisible) {
      shortcut = untrack(() => session.preferences.shortcut);
      saved = false;
      onerror('');
    }
  });
  async function place(toolbarPosition: ToolbarPosition) {
    placing = true;
    try { await savePreferences({ ...session.preferences, toolbarPosition }); }
    catch (e) { onerror(e); }
    finally { placing = false; }
  }
  async function save() {
    saving = true; saved = false; onerror('');
    try { await savePreferences({ ...session.preferences, shortcut: shortcut.trim() }); saved = true; }
    catch (e) { onerror(e); }
    finally { saving = false; }
  }
</script>

<main class="settings-window">
  <section class="settings-card" aria-label="Glassboard settings">
    <header>
      <div><h1>Settings</h1><p>Glassboard</p></div>
      <button class="icon-button" title="Close settings" aria-label="Close settings" onclick={() => action('close-settings').catch(onerror)}><X size={17}/></button>
    </header>
    <div class="controls" role="group" aria-label="Annotation controls">
      <button class="control-row" role="switch" aria-checked={session.mode !== 'hidden'} onclick={() => run('toggle')}>
        <span>Show annotations</span><span class="switch" class:on={session.mode !== 'hidden'} aria-hidden="true"></span>
      </button>
      <button class="control-row" role="switch" aria-checked={session.mode === 'draw'} disabled={session.mode === 'hidden'} onclick={() => run('interact')}>
        <span>Drawing mode</span><kbd>{shortcutLabel('CommandOrControl+Shift+I')}</kbd><span class="switch" class:on={session.mode === 'draw'} aria-hidden="true"></span>
      </button>
      <button class="control-row" role="switch" aria-checked={session.mode !== 'hidden' && session.toolbarVisible} disabled={session.mode === 'hidden'} onclick={() => run('toolbar')}>
        <span>Show toolbar</span><kbd>{shortcutLabel('CommandOrControl+Shift+H')}</kbd><span class="switch" class:on={session.mode !== 'hidden' && session.toolbarVisible} aria-hidden="true"></span>
      </button>
    </div>
    <div class="placement">
      <div class="placement-label">Toolbar position</div>
      <div class="positions" role="group" aria-label="Toolbar position">
        {#each ['left', 'right', 'bottom'] as position}
          <button class:chosen={session.preferences.toolbarPosition === position} aria-pressed={session.preferences.toolbarPosition === position} disabled={placing} onclick={() => place(position as ToolbarPosition)}>{position[0].toUpperCase() + position.slice(1)}</button>
        {/each}
      </div>
    </div>
    <form onsubmit={event => { event.preventDefault(); save(); }}>
      <label for="shortcut">Show / hide shortcut</label>
      <div class="shortcut-editor">
        <input id="shortcut" bind:value={shortcut} oninput={() => saved = false} spellcheck="false" autocomplete="off" aria-describedby="shortcut-hint"/>
        <button class="save" type="submit" disabled={saving}>{saving ? 'Saving…' : saved ? 'Saved' : 'Save'}</button>
      </div>
      <p id="shortcut-hint">For example: CommandOrControl+Shift+A</p>
    </form>
    <footer>
      <button onclick={() => run('clear-all')}><Trash2 size={14}/>Clear all drawings</button>
      <button onclick={() => run('quit')}><Power size={14}/>Quit Glassboard</button>
    </footer>
    <div class="feedback" aria-live="polite">
      {#if error}<p class="error" role="alert">{error}</p>{:else if saved}<p>Shortcut saved.</p>{/if}
    </div>
  </section>
</main>

<style>
  .settings-window { color-scheme: light dark; padding: 8px; width: 100%; height: 100vh; }
  .settings-card { height: 100%; overflow-y: auto; padding: 18px; border: 1px solid var(--border); border-radius: 14px; background: var(--surface); box-shadow: 0 2px 7px var(--shadow); }
  header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 22px; }
  h1 { font-size: 17px; line-height: 1.2; letter-spacing: -.3px; font-weight: 600; margin: 0; }
  header p { margin: 4px 0 0; font-size: 11px; color: var(--muted); }
  header .icon-button { width: 26px; height: 26px; margin: -5px -5px 0 0; }
  label { display: block; font-size: 12px; font-weight: 500; margin-bottom: 9px; }
  .placement { margin-bottom: 20px; }
  .placement-label { font-size: 12px; font-weight: 500; margin-bottom: 9px; }
  .positions { display: flex; gap: 4px; padding: 3px; background: var(--subtle-surface); border: 1px solid var(--border); border-radius: 8px; }
  .positions button { flex: 1; height: 28px; border-radius: 5px; font-size: 11px; color: var(--secondary-text); }
  .positions button.chosen { background: var(--selected); color: var(--strong-text); }
  .positions button:hover:not(:disabled) { background: var(--hover); }
  .shortcut-editor { display: flex; gap: 7px; }
  input { flex: 1; min-width: 0; padding: 8px; border: 1px solid var(--border); border-radius: 7px; background: var(--input-surface); color: var(--text); font-size: 10px; -webkit-user-select: text; user-select: text; }
  .save { padding: 0 11px; border-radius: 7px; background: var(--save-surface); font-size: 11px; }
  #shortcut-hint { margin: 7px 0 0; font-size: 10px; color: var(--muted); }
  .controls { display: grid; gap: 2px; padding-bottom: 16px; margin-bottom: 18px; border-bottom: 1px solid var(--divider); }
  .control-row { display: flex; align-items: center; gap: 9px; width: 100%; height: 34px; padding: 0; font-size: 12px; text-align: left; border-radius: 6px; }
  .control-row > span:first-child { flex: 1; }
  .control-row:hover:not(:disabled) { color: var(--strong-text); }
  .control-row kbd { font-size: 10px; }
  .switch { width: 28px; height: 16px; border-radius: 10px; background: var(--selected); box-shadow: inset 0 0 0 1px var(--border); }
  .switch::after { content: ''; display: block; width: 12px; height: 12px; margin: 2px; border-radius: 50%; background: var(--surface); box-shadow: 0 1px 2px var(--shadow); transition: transform .12s; }
  .switch.on { background: var(--focus-ring); }
  .switch.on::after { transform: translateX(12px); }
  footer { display: flex; justify-content: space-between; border-top: 1px solid var(--divider); padding-top: 15px; margin-top: 20px; }
  footer button { display: flex; align-items: center; gap: 6px; padding: 0; font-size: 11px; color: var(--secondary-text); }
  footer button:hover { color: var(--strong-text); }
  .feedback { margin-top: 12px; font-size: 11px; line-height: 1.4; color: var(--muted); }
  .feedback p { margin: 0; }
  .feedback .error { color: var(--error-text); overflow-wrap: anywhere; }
</style>
