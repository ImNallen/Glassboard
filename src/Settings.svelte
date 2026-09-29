<script lang="ts">
  import { X, Trash2, Power, RotateCcw } from '@lucide/svelte';
  import { action, mac, native, savePreferences, shortcutLabel, type ToolbarPosition, type Session } from './lib/session';
  import { DEFAULT_SHORTCUT, heldModifiers, MODIFIER_KEYS, recordShortcut } from './lib/shortcuts';
  let { session, error, onerror }: { session: Session; error: string; onerror: (error: unknown) => void } = $props();
  let recording = $state(false);
  let pending = $state<string[]>([]);
  let saving = $state(false);
  let saved = $state(false);
  let placing = $state(false);
  let windowVisible = $derived(session.settingsOpen || !native);
  let shortcut = $derived(session.preferences.shortcut);
  let pendingLabel = $derived(pending.length ? shortcutLabel(pending.join('+')) + (mac ? '' : '+') : '');
  const run = (name: string) => action(name).catch(onerror);
  $effect(() => {
    if (windowVisible) { saved = false; onerror(''); }
    else stopRecording();
  });
  async function place(toolbarPosition: ToolbarPosition) {
    placing = true;
    try { await savePreferences({ ...session.preferences, toolbarPosition }); }
    catch (e) { onerror(e); }
    finally { placing = false; }
  }
  function startRecording() { recording = true; pending = []; saved = false; onerror(''); }
  function stopRecording() { recording = false; pending = []; }
  async function save(next: string) {
    if (next === shortcut) return;
    saving = true; saved = false; onerror('');
    try { await savePreferences({ ...session.preferences, shortcut: next }); saved = true; }
    catch (e) { onerror(e); }
    finally { saving = false; }
  }
  function recordKey(event: KeyboardEvent) {
    if (!recording) return;
    event.preventDefault(); event.stopPropagation();
    if (event.key === 'Escape') { stopRecording(); return; }
    if (MODIFIER_KEYS.has(event.key)) { pending = heldModifiers(event, mac); return; }
    const next = recordShortcut(event, mac);
    if (!next) { onerror('That key cannot be used in a shortcut.'); return; }
    stopRecording();
    save(next);
  }
  function releaseKey(event: KeyboardEvent) {
    if (recording && MODIFIER_KEYS.has(event.key)) pending = heldModifiers(event, mac);
  }
</script>

<main class="settings-window">
  <section class="settings-card" aria-label="Glassboard settings">
    <header>
      <div><h1>Settings</h1><p>Glassboard</p></div>
      <button class="icon-button" title="Close settings" aria-label="Close settings" onclick={() => action('close-settings').catch(onerror)}><X size={17}/></button>
    </header>
    <div class="shortcut-setting">
      <div class="placement-label" id="shortcut-label">Annotate / work shortcut</div>
      <div class="shortcut-editor">
        <button class="recorder" class:recording aria-labelledby="shortcut-label" aria-describedby="shortcut-hint" disabled={saving}
          onclick={() => recording ? stopRecording() : startRecording()} onkeydown={recordKey} onkeyup={releaseKey} onblur={stopRecording}>
          {#if recording}<span class="pending">{pendingLabel || 'Press keys…'}</span>{:else}<kbd>{shortcutLabel(shortcut)}</kbd>{/if}
        </button>
        <button class="icon-button" title="Reset to default" aria-label="Reset shortcut to default" disabled={saving || shortcut === DEFAULT_SHORTCUT} onclick={() => save(DEFAULT_SHORTCUT)}><RotateCcw size={15}/></button>
      </div>
      <p id="shortcut-hint">{recording ? `Include ${mac ? '⌘, ⌃, or ⌥' : 'Ctrl, Win, or Alt'}. Escape cancels.` : 'Switches between annotation and work. Each annotation session starts clear.'}</p>
    </div>
    <div class="placement">
      <div class="placement-label">Toolbar position</div>
      <div class="positions" role="group" aria-label="Toolbar position">
        {#each ['left', 'right', 'bottom'] as position}
          <button class:chosen={session.preferences.toolbarPosition === position} aria-pressed={session.preferences.toolbarPosition === position} disabled={placing} onclick={() => place(position as ToolbarPosition)}>{position[0].toUpperCase() + position.slice(1)}</button>
        {/each}
      </div>
    </div>
    <button class="tutorial-link" onclick={() => run('replay-tutorial')}>Show tutorial</button>
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
  .placement { margin-bottom: 20px; }
  .placement-label { font-size: 12px; font-weight: 500; margin-bottom: 9px; }
  .positions { display: flex; gap: 4px; padding: 3px; background: var(--subtle-surface); border: 1px solid var(--border); border-radius: 8px; }
  .positions button { flex: 1; height: 28px; border-radius: 5px; font-size: 11px; color: var(--secondary-text); }
  .positions button.chosen { background: var(--selected); color: var(--strong-text); }
  .positions button:hover:not(:disabled) { background: var(--hover); }
  .shortcut-setting { margin-bottom: 20px; }
  .shortcut-editor { display: flex; gap: 7px; align-items: center; }
  .recorder { flex: 1; min-width: 0; height: 34px; padding: 0 10px; display: flex; align-items: center; border: 1px solid var(--border); border-radius: 7px; background: var(--input-surface); color: var(--text); font-size: 12px; text-align: left; }
  .recorder:hover:not(:disabled) { background: var(--hover); }
  .recorder.recording { border-color: var(--focus-ring); box-shadow: 0 0 0 2px color-mix(in srgb, var(--focus-ring) 25%, transparent); }
  .recorder kbd { font-size: 12px; }
  .pending { color: var(--muted); }
  .shortcut-editor .icon-button { width: 30px; height: 30px; }
  #shortcut-hint { margin: 7px 0 0; font-size: 10px; color: var(--muted); }
  .tutorial-link { padding: 0; font-size: 12px; text-decoration: underline; text-underline-offset: 3px; }
  footer { display: flex; justify-content: space-between; border-top: 1px solid var(--divider); padding-top: 15px; margin-top: 20px; }
  footer button { display: flex; align-items: center; gap: 6px; padding: 0; font-size: 11px; color: var(--secondary-text); }
  footer button:hover { color: var(--strong-text); }
  .feedback { margin-top: 12px; font-size: 11px; line-height: 1.4; color: var(--muted); }
  .feedback p { margin: 0; }
  .feedback .error { color: var(--error-text); overflow-wrap: anywhere; }
</style>
