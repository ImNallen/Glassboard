<script lang="ts">
  import { Copy, EyeOff, Keyboard, Palette, Power, Redo2, RotateCcw, Scan, SlidersHorizontal, Trash2, Undo2, X } from '@lucide/svelte';
  import { untrack } from 'svelte';
  import GitHubIcon from '@glassboard/ui/GitHubIcon.svelte';
  import Logo from '@glassboard/ui/Logo.svelte';
  import { action, native, savePreferences, type Preferences, type Session, type ToolbarPosition, type Action } from '@glassboard/ui/session';
  import { COLOR_SHORTCUTS, DEFAULT_SHORTCUT, KEYBINDING_GROUPS, KEYBINDINGS, keybinding, platformMac, rebindCommand, rebindToggle, sameShortcut, shortcutLabel, TOOL_SHORTCUTS } from '@glassboard/ui/shortcuts';
  import { AUTO_FADE_OPTIONS } from '@glassboard/ui/drawing';
  import { rainbowPreview, shiftingPreview, swatchColor, swatchName } from '@glassboard/ui/swatches';
  import { TOOL_ICONS } from '@glassboard/ui/tool-icons';
  import type { Errors } from '@glassboard/ui/errors';
  import ColorSettings from './ColorSettings.svelte';
  import ShortcutRecorder from './ShortcutRecorder.svelte';
  import { getAutostart, setAutostart } from './lib/autostart';
  let { session, errors }: { session: Session; errors: Errors } = $props();
  type Tab = 'general' | 'colors' | 'keybindings';
  const TABS: readonly { id: Tab; label: string; icon: typeof Keyboard }[] = [
    { id: 'general', label: 'General', icon: SlidersHorizontal },
    { id: 'colors', label: 'Colors', icon: Palette },
    { id: 'keybindings', label: 'Keybindings', icon: Keyboard },
  ];
  const POSITIONS: readonly { id: ToolbarPosition; label: string }[] = [{ id: 'left', label: 'Left' }, { id: 'bottom', label: 'Bottom' }, { id: 'right', label: 'Right' }];
  const ICONS: Record<string, typeof Keyboard> = { undo: Undo2, redo: Redo2, hide: EyeOff, capture: Scan, copy: Copy, ...Object.fromEntries(TOOL_SHORTCUTS.map(tool => [tool.command, TOOL_ICONS[tool.id]])) };
  const REPOSITORY_URL = 'https://github.com/ImNallen/Glassboard';
  const GLOBAL_HINT = `Include ${platformMac ? '⌘, ⌃, or ⌥' : 'Ctrl, Win, or Alt'}. Esc cancels.`;
  const LOCAL_HINT = `Press a key or combination. Esc cancels, ${platformMac ? '⌫' : 'Backspace'} clears.`;
  let tab = $state<Tab>('general');
  let panel = $state<HTMLDivElement>();
  let busy = $state(false);
  let notice = $state('');
  let hint = $state('');
  let noticeTimer: ReturnType<typeof setTimeout>;
  let windowVisible = $derived(session.settingsOpen || !native);
  let preferences = $derived(session.preferences);
  let customized = $derived(Object.keys(preferences.keybindings).length > 0 || !sameShortcut(preferences.shortcut, DEFAULT_SHORTCUT, platformMac));
  let fade = $derived(preferences.autoFadeSeconds);
  // Read from the OS each time settings opens, since it can be changed in System Settings.
  let autostart = $state<boolean | null>(null);
  let switchingAutostart = $state(false);
  $effect(() => { if (windowVisible) getAutostart().then(enabled => autostart = enabled).catch(errors.report); });
  // Color rows in Keybindings show each swatch's current color and name.
  let swatches = $derived(Object.fromEntries(COLOR_SHORTCUTS.map(choice => [choice.command,
    choice.slot !== undefined ? { background: swatchColor(preferences.swatches, choice.slot), name: swatchName(preferences.swatches, choice.slot) }
      : { background: choice.colorMode === 'rainbow' ? rainbowPreview(preferences.rainbowColors) : shiftingPreview(preferences.cycleColors), name: choice.name }])));
  const run = (name: Action) => action(name).catch(errors.report);
  $effect(() => {
    if (!windowVisible) return;
    notice = ''; errors.clear();
    // Open where the unavailable shortcut can be replaced.
    if (untrack(() => session.shortcutUnavailable)) tab = 'keybindings';
  });
  $effect(() => () => clearTimeout(noticeTimer));

  function select(next: Tab) {
    tab = next;
    if (panel) panel.scrollTop = 0;
  }
  function tabKeydown(event: KeyboardEvent) {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    event.preventDefault();
    const step = event.key === 'ArrowRight' ? 1 : TABS.length - 1;
    const next = TABS[(TABS.findIndex(item => item.id === tab) + step) % TABS.length].id;
    select(next);
    document.getElementById(`tab-${next}`)?.focus();
  }
  async function update(patch: Partial<Preferences>, message = '') {
    busy = true; notice = ''; errors.clear();
    clearTimeout(noticeTimer);
    try {
      await savePreferences({ ...session.preferences, ...patch });
      notice = message;
      if (message) noticeTimer = setTimeout(() => notice = '', 4000);
    }
    catch (e) { errors.report(e); }
    finally { busy = false; }
  }
  function setToggle(next: string) {
    const result = rebindToggle(preferences, next);
    if (result) update({ shortcut: result.shortcut, keybindings: result.keybindings }, result.moved.length ? `Unbound ${result.moved.join(', ')} to make room.` : 'Shortcut saved.');
  }
  function setBinding(id: string, next: string) {
    const result = rebindCommand(preferences, id, next);
    if (result === 'toggle') errors.report(`${shortcutLabel(next)} is already used to toggle Glassboard.`);
    else if (result) update({ keybindings: result.keybindings }, result.moved.length ? `Moved ${shortcutLabel(next)} from ${result.moved.join(', ')}.` : '');
  }
  /** Native webviews don't open links themselves; Rust opens the fixed URL behind each action. */
  function openLink(event: MouseEvent, name: Action) {
    if (!native) return;
    event.preventDefault();
    run(name);
  }
  async function toggleAutostart() {
    if (autostart === null) return;
    switchingAutostart = true; errors.clear();
    try { autostart = await setAutostart(!autostart); }
    catch (e) { errors.report(e); }
    finally { switchingAutostart = false; }
  }
  const fadeNote = (seconds: number) => seconds === 0 ? 'Marks stay until you return to work.' : `Marks fade ${seconds} seconds after you draw them.`;
</script>

<main class="settings-window">
  <section class="settings" aria-label="Glassboard settings">
    <header>
      <span class="brand"><Logo size={18}/>Glassboard<span class="version">v{__APP_VERSION__}</span></span>
      <div class="header-actions">
        <a class="icon-link" href={REPOSITORY_URL} target="_blank" rel="noreferrer" title="Glassboard on GitHub" aria-label="Glassboard on GitHub"
          onclick={event => openLink(event, 'open-github')}><GitHubIcon/></a>
        <button class="icon-link" title="Close" aria-label="Close settings" onclick={() => run('close-settings')}><X size={15}/></button>
      </div>
    </header>

    <div class="tabs" role="tablist" aria-label="Settings sections">
      {#each TABS as item}
        <button role="tab" id={`tab-${item.id}`} aria-selected={tab === item.id} aria-controls="settings-panel" tabindex={tab === item.id ? 0 : -1}
          onclick={() => select(item.id)} onkeydown={tabKeydown}><item.icon size={13} strokeWidth={2}/>{item.label}</button>
      {/each}
    </div>

    <div class="panel" id="settings-panel" role="tabpanel" aria-labelledby={`tab-${tab}`} bind:this={panel}>
      {#if tab === 'general'}
        <section class="group" aria-labelledby="position-title">
          <h2 id="position-title">Toolbar position</h2>
          <div class="positions" role="group" aria-labelledby="position-title">
            {#each POSITIONS as position}
              <button class="position" class:chosen={preferences.toolbarPosition === position.id} aria-pressed={preferences.toolbarPosition === position.id}
                disabled={busy} onclick={() => update({ toolbarPosition: position.id })}>
                <span class={`screen ${position.id}`} aria-hidden="true"><span></span></span>{position.label}
              </button>
            {/each}
          </div>
        </section>

        <section class="group" aria-labelledby="fade-title">
          <h2 id="fade-title">Auto-fade</h2>
          <div class="segmented" role="group" aria-labelledby="fade-title">
            {#each AUTO_FADE_OPTIONS as seconds}
              <button class:chosen={fade === seconds} aria-pressed={fade === seconds} disabled={busy} onclick={() => update({ autoFadeSeconds: seconds })}>{seconds ? `${seconds}s` : 'Off'}</button>
            {/each}
          </div>
          <p class="note">{fadeNote(fade)}</p>
        </section>

        <section class="group" aria-labelledby="startup-title">
          <h2 id="startup-title">Startup</h2>
          <div class="list">
            <div class="row">
              <span class="row-label" id="autostart-label">{platformMac ? 'Open at login' : 'Start with Windows'}<small>Glassboard waits in the {platformMac ? 'menu bar' : 'tray'} until you need it</small></span>
              <button class="switch" role="switch" aria-checked={autostart === true} aria-labelledby="autostart-label"
                disabled={autostart === null || switchingAutostart} onclick={toggleAutostart}><span></span></button>
            </div>
          </div>
        </section>

        <section class="group" aria-labelledby="help-title">
          <h2 id="help-title">Help</h2>
          <div class="list">
            <div class="row">
              <span class="row-label">Tutorial<small>A quick walkthrough of the basics</small></span>
              <button class="button" onclick={() => run('replay-tutorial')}>Replay</button>
            </div>
            <div class="row">
              <span class="row-label">Feedback<small>Found a bug or have an idea? Open an issue.</small></span>
              <a class="button" href={`${REPOSITORY_URL}/issues/new`} target="_blank" rel="noreferrer" title="Opens GitHub Issues in your browser"
                onclick={event => openLink(event, 'report-issue')}>Report</a>
            </div>
          </div>
        </section>
      {:else if tab === 'colors'}
        <ColorSettings {preferences} onsave={update}/>
      {:else}
        <div class="intro">
          <p>Click a row, then press new keys.</p>
          <button class="text-button reset-all" disabled={busy || !customized} onclick={() => update({ shortcut: DEFAULT_SHORTCUT, keybindings: {} }, 'All keybindings reset.')}>Reset all<RotateCcw size={12}/></button>
        </div>

        <section class="group" aria-labelledby="keys-global">
          <h2 id="keys-global">Global</h2>
          <div class="list">
            <div class="row binding">
              <ShortcutRecorder label="Toggle Glassboard" shortcut={preferences.shortcut} fallback={DEFAULT_SHORTCUT} physical active={windowVisible} disabled={busy}
                onchange={setToggle} {errors} onrecord={recording => hint = recording ? GLOBAL_HINT : ''}>
                <span class="row-icon"><Logo size={14}/></span>
                <span class="row-label">Toggle Glassboard</span>
              </ShortcutRecorder>
            </div>
          </div>
          {#if session.shortcutUnavailable}<p class="note unavailable" role="alert">Another app is using {shortcutLabel(preferences.shortcut)}, so it can’t turn on drawing. Record a different shortcut.</p>{/if}
        </section>

        {#each KEYBINDING_GROUPS as group}
          <section class="group" aria-labelledby={`keys-${group}`}>
            <h2 id={`keys-${group}`}>{group}</h2>
            <div class="list">
              {#each KEYBINDINGS.filter(binding => binding.group === group) as binding (binding.id)}
                {@const Icon = ICONS[binding.id]}
                {@const swatch = swatches[binding.id]}
                <div class="row binding">
                  <ShortcutRecorder label={swatch?.name ?? binding.name} shortcut={keybinding(preferences.keybindings, binding.id)} fallback={binding.shortcut} clearable active={windowVisible} disabled={busy}
                    onchange={next => setBinding(binding.id, next)} {errors} onrecord={recording => hint = recording ? LOCAL_HINT : ''}>
                    <span class="row-icon">{#if swatch}<span class="swatch" style:background={swatch.background}></span>{:else if Icon}<Icon size={15} strokeWidth={1.9}/>{/if}</span>
                    <span class="row-label">{swatch?.name ?? binding.name}{#if binding.description}<small>{binding.description}</small>{/if}</span>
                  </ShortcutRecorder>
                </div>
              {/each}
            </div>
          </section>
        {/each}
      {/if}
    </div>

    <div class="status" aria-live="polite">
      {#if errors.message}<p class="error" role="alert">{errors.message}<button class="dismiss" aria-label="Dismiss error" onclick={errors.dismiss}><X size={12}/></button></p>
      {:else if hint}<p>{hint}</p>
      {:else if notice}<p>{notice}</p>{/if}
    </div>

    <footer>
      <button onclick={() => run('clear-all')}><Trash2 size={13}/>Clear all drawings</button>
      <button onclick={() => run('quit')}><Power size={13}/>Quit</button>
    </footer>
  </section>
</main>

<style>
  .settings-window { color-scheme: light dark; padding: 8px; width: 100%; height: 100vh; }
  .settings { --keycap: #ffffff; --raised: #ffffff; --reset: #d33d3d; display: flex; flex-direction: column; height: 100%; overflow: hidden; border: 1px solid var(--gb-border); border-radius: 14px; background: var(--gb-surface); box-shadow: 0 2px 7px var(--gb-shadow); }
  @media (prefers-color-scheme: dark) { .settings { --keycap: #3a3d44; --raised: #3d4048; --reset: #ff6b6b; } }

  header { display: flex; align-items: center; justify-content: space-between; padding: 14px 12px 0 16px; }
  .brand { display: flex; align-items: center; gap: 7px; font-size: 14px; font-weight: 600; letter-spacing: -.2px; color: var(--gb-strong-text); }
  .version { margin-left: 1px; padding: 2px 6px; border-radius: 999px; background: color-mix(in srgb, var(--gb-text) 7%, transparent); font-size: 10px; font-weight: 500; letter-spacing: 0; color: var(--gb-muted); font-variant-numeric: tabular-nums; }
  .header-actions { display: flex; gap: 2px; }
  .icon-link { display: grid; place-items: center; width: 26px; height: 26px; border-radius: 7px; color: var(--gb-muted); }
  .icon-link:hover { background: var(--gb-hover); color: var(--gb-strong-text); }
  .icon-link:focus-visible { outline: 2px solid var(--gb-focus-ring); outline-offset: 2px; }

  .tabs, .segmented { display: flex; gap: 2px; padding: 3px; border-radius: 9px; background: color-mix(in srgb, var(--gb-text) 6%, transparent); }
  .tabs { margin: 14px 14px 0; }
  .tabs button, .segmented button { flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px; height: 26px; border-radius: 6px; font-size: 12px; font-weight: 500; color: var(--gb-secondary-text); transition: background .12s, color .12s, box-shadow .12s; }
  .tabs button:hover, .segmented button:hover:not(:disabled) { color: var(--gb-strong-text); }
  .tabs button[aria-selected="true"], .segmented button.chosen { background: var(--raised); color: var(--gb-strong-text); box-shadow: 0 0 0 1px var(--gb-selected-border), 0 1px 2px var(--gb-shadow); }
  .segmented button { font-size: 11.5px; font-variant-numeric: tabular-nums; }

  .panel { flex: 1; min-height: 0; overflow-y: auto; padding: 16px 14px 12px; scrollbar-width: thin; }
  /* Section styles are shared with tab components such as ColorSettings. */
  .panel :global(.group + .group) { margin-top: 20px; }
  .panel :global(h2) { margin: 0 0 8px 2px; font-size: 10px; font-weight: 600; letter-spacing: .8px; text-transform: uppercase; color: var(--gb-muted); }
  .note { margin: 8px 2px 0; font-size: 11px; color: var(--gb-muted); }

  .positions { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .position { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 12px 6px 9px; border-radius: 10px; border: 1px solid var(--gb-divider); background: var(--gb-subtle-surface); font-size: 11.5px; color: var(--gb-secondary-text); transition: border-color .12s, background .12s, color .12s; }
  .position:hover:not(:disabled) { background: var(--gb-hover); color: var(--gb-strong-text); }
  .position.chosen { border-color: var(--gb-focus-ring); box-shadow: inset 0 0 0 1px var(--gb-focus-ring); color: var(--gb-strong-text); }
  .screen { position: relative; width: 58px; height: 38px; border-radius: 6px; border: 1px solid var(--gb-border); background: var(--gb-input-surface); }
  .screen span { position: absolute; border-radius: 2px; background: var(--gb-muted); opacity: .55; transition: background .12s, opacity .12s; }
  .screen.bottom span { left: 50%; bottom: 4px; width: 26px; height: 4px; translate: -50% 0; }
  .screen.left span, .screen.right span { top: 50%; width: 4px; height: 20px; translate: 0 -50%; }
  .screen.left span { left: 4px; }
  .screen.right span { right: 4px; }
  .chosen .screen span { background: var(--gb-focus-ring); opacity: 1; }

  .list { overflow: hidden; border: 1px solid var(--gb-divider); border-radius: 10px; background: var(--gb-subtle-surface); }
  .row { display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 5px 6px 5px 12px; }
  .row + .row { border-top: 1px solid var(--gb-divider); }
  /* The recorder button fills binding rows so the whole row is clickable. */
  .row.binding { position: relative; padding: 0; }
  .row-icon { display: grid; place-items: center; width: 16px; flex-shrink: 0; color: var(--gb-icon); }
  .swatch { width: 13px; height: 13px; border-radius: 50%; box-shadow: inset 0 0 0 1px var(--gb-swatch-border); }
  .row-label { flex: 1; min-width: 0; font-size: 12.5px; color: var(--gb-text); }
  .row-label small { display: block; margin-top: 1px; font-size: 10.5px; color: var(--gb-muted); }
  .button { display: inline-flex; align-items: center; height: 26px; margin-right: 2px; padding: 0 11px; color: inherit; text-decoration: none; border-radius: 7px; background: var(--raised); box-shadow: 0 0 0 1px var(--gb-border), 0 1px 1px var(--gb-shadow); font-size: 11.5px; font-weight: 500; }
  .button:focus-visible { outline: 2px solid var(--gb-focus-ring); outline-offset: 2px; }
  .switch { position: relative; flex-shrink: 0; width: 34px; height: 20px; margin-right: 4px; padding: 0; border-radius: 10px; background: color-mix(in srgb, var(--gb-text) 18%, transparent); transition: background .16s; }
  .switch span { position: absolute; top: 2px; left: 2px; width: 16px; height: 16px; border-radius: 50%; background: #fff; box-shadow: 0 1px 2px #0000004d; transition: translate .16s ease; }
  .switch[aria-checked="true"] { background: var(--gb-focus-ring); }
  .switch[aria-checked="true"] span { translate: 14px 0; }
  .switch:disabled { opacity: .5; }
  @media (prefers-reduced-motion: reduce) { .switch, .switch span { transition: none; } }
  .button:hover { color: var(--gb-strong-text); box-shadow: 0 0 0 1px var(--gb-selected-border), 0 1px 2px var(--gb-shadow); }

  .panel :global(.intro) { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin: 0 2px 16px; }
  .panel :global(.intro p) { margin: 0; font-size: 11.5px; color: var(--gb-muted); }
  .panel :global(.text-button) { display: flex; align-items: center; gap: 5px; flex-shrink: 0; padding: 4px 6px; margin-right: 5px; border-radius: 6px; font-size: 11.5px; font-weight: 500; color: var(--gb-secondary-text); }
  .panel :global(.text-button:hover:not(:disabled)) { background: var(--gb-hover); color: var(--gb-strong-text); }
  .panel :global(.reset-all) { color: var(--reset); }
  .panel :global(.reset-all:hover:not(:disabled)) { background: color-mix(in srgb, var(--reset) 14%, transparent); color: var(--reset); }
  .panel :global(.reset-all:disabled) { color: var(--gb-muted); }

  .status { padding: 0 16px; font-size: 11px; line-height: 1.4; color: var(--gb-muted); }
  .status p { margin: 0 0 9px; }
  .status .error { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; color: var(--gb-error-text); overflow-wrap: anywhere; }
  .status .dismiss { display: grid; place-items: center; flex-shrink: 0; width: 18px; height: 18px; border-radius: 5px; color: var(--gb-muted); }
  .status .dismiss:hover { background: var(--gb-hover); color: var(--gb-strong-text); }
  .note.unavailable { color: var(--gb-error-text); }
  footer { display: flex; justify-content: space-between; padding: 10px 12px; border-top: 1px solid var(--gb-divider); }
  footer button { display: flex; align-items: center; gap: 6px; padding: 5px 6px; border-radius: 6px; font-size: 11.5px; color: var(--gb-secondary-text); }
  footer button:hover { background: var(--gb-hover); color: var(--gb-strong-text); }
</style>
