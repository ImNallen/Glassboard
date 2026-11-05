<script lang="ts">
  import { CircleArrowUp, Copy, EyeOff, Keyboard, Languages, LoaderCircle, Minus, Palette, Plus, Power, Redo2, RefreshCw, RotateCcw, Scan, Search, SlidersHorizontal, Undo2, X } from '@lucide/svelte';
  import { untrack } from 'svelte';
  import GitHubIcon from '@glassboard/ui/GitHubIcon.svelte';
  import Logo from '@glassboard/ui/Logo.svelte';
  import { list, locale, number, t, type Message } from '@glassboard/ui/i18n';
  import { action, native, savePreferences, type Preferences, type Session, type ToolbarPosition, type Action } from '@glassboard/ui/session';
  import { COLOR_SHORTCUTS, DEFAULT_CAPTURE_SHORTCUT, DEFAULT_SHORTCUT, type GlobalCommand, KEYBINDING_GROUPS, KEYBINDINGS, keybinding, platformMac, rebindCommand, rebindGlobal, sameShortcut, shortcutLabel, TOOL_SHORTCUTS } from '@glassboard/ui/shortcuts';
  import { AUTO_FADE_OPTIONS } from '@glassboard/ui/drawing';
  import { rainbowPreview, shiftingPreview, swatchColor, swatchName } from '@glassboard/ui/swatches';
  import { TOOL_ICONS } from '@glassboard/ui/tool-icons';
  import type { Errors } from '@glassboard/ui/errors';
  import ColorSettings from './ColorSettings.svelte';
  import LanguagePicker from './LanguagePicker.svelte';
  import ShortcutRecorder from './ShortcutRecorder.svelte';
  import { getAutostart, setAutostart } from './lib/autostart';
  import { suspendGlobalShortcuts } from './lib/global-shortcuts';
  import { getResourceUsage, usageLabel, type ResourceUsage } from './lib/usage';
  import { checkForUpdates, installUpdate, updateDetail, updateLabel, watchUpdates, type UpdateStatus } from './lib/updates';
  let { session, errors }: { session: Session; errors: Errors } = $props();
  type Tab = 'general' | 'colors' | 'keybindings';
  const TABS: readonly { id: Tab; label: Message; icon: typeof Keyboard }[] = [
    { id: 'general', label: 'General', icon: SlidersHorizontal },
    { id: 'colors', label: 'Colors', icon: Palette },
    { id: 'keybindings', label: 'Keybindings', icon: Keyboard },
  ];
  const POSITIONS: readonly { id: ToolbarPosition; label: Message }[] = [{ id: 'left', label: 'Left' }, { id: 'bottom', label: 'Bottom' }, { id: 'right', label: 'Right' }];
  const ICONS: Record<string, typeof Keyboard> = { undo: Undo2, redo: Redo2, hide: EyeOff, 'size-smaller': Minus, 'size-larger': Plus, capture: Scan, copy: Copy, ...Object.fromEntries(TOOL_SHORTCUTS.map(tool => [tool.command, TOOL_ICONS[tool.id]])) };
  const REPOSITORY_URL = 'https://github.com/ImNallen/Glassboard';
  const modifierChoices = () => list(platformMac ? ['⌘', '⌃', '⌥'] : [t('Ctrl'), t('Win'), t('Alt')], 'disjunction');
  const clearKey = () => platformMac ? '⌫' : t('Backspace');
  const globalHint = () => t('Include {keys}. Esc cancels.', { keys: modifierChoices() });
  const captureHint = () => t('Include {keys}. Esc cancels, {clear} clears.', { keys: modifierChoices(), clear: clearKey() });
  const localHint = () => t('Press a key or combination. Esc cancels, {clear} clears.', { clear: clearKey() });
  const ALREADY_USED = { toggle: '{shortcut} is already used to toggle Glassboard.', capture: '{shortcut} is already used to take a screenshot.' } as const;
  let tab = $state<Tab>('general');
  let panel = $state<HTMLDivElement>();
  let shortcutSearch = $state('');
  let onlyCustomized = $state(false);
  let busy = $state(false);
  let notice = $state('');
  let hint = $state('');
  let noticeTimer: ReturnType<typeof setTimeout>;
  let windowVisible = $derived(session.settingsOpen || !native);
  let preferences = $derived(session.preferences);
  const GLOBAL_BINDINGS: readonly { command: GlobalCommand; label: Message; fallback: string }[] = [
    { command: 'toggle', label: 'Toggle Glassboard', fallback: DEFAULT_SHORTCUT },
    { command: 'capture', label: 'Take screenshot', fallback: DEFAULT_CAPTURE_SHORTCUT },
  ];
  let globalBindings = $derived(GLOBAL_BINDINGS.map(binding => ({ ...binding, label: t(binding.label),
    shortcut: binding.command === 'toggle' ? preferences.shortcut : preferences.captureShortcut,
  })));
  let customized = $derived(globalBindings.some(binding => !sameShortcut(binding.shortcut, binding.fallback, platformMac))
    || KEYBINDINGS.some(binding => !sameShortcut(keybinding(preferences.keybindings, binding.id), binding.shortcut, platformMac)));
  let visibleGlobals = $derived(globalBindings.filter(binding => matchesBinding({ ...binding, group: t('Global') })));
  let visibleGroups = $derived(KEYBINDING_GROUPS.map(group => ({ name: group, bindings: KEYBINDINGS
    .filter(binding => binding.group === group)
    .map(binding => ({ ...binding, group: t(group), label: swatches[binding.id]?.name ?? t(binding.name), description: binding.description && t(binding.description), fallback: binding.shortcut, shortcut: keybinding(preferences.keybindings, binding.id) }))
    .filter(matchesBinding),
  })).filter(group => group.bindings.length));
  let fade = $derived(preferences.autoFadeSeconds);
  // Read from the OS each time settings opens, since it can be changed in System Settings.
  let autostart = $state<boolean | null>(null);
  let switchingAutostart = $state(false);
  $effect(() => { if (windowVisible) getAutostart().then(enabled => autostart = enabled).catch(errors.report); });
  let usage = $state<ResourceUsage | null>(null);
  $effect(() => {
    if (!windowVisible) return;
    const poll = () => getResourceUsage().then(next => usage = next).catch(error => { clearInterval(timer); errors.report(error); });
    poll();
    const timer = setInterval(poll, 2000);
    return () => clearInterval(timer);
  });
  let updateStatus = $state<UpdateStatus>({ state: 'idle' });
  let updating = $derived(updateStatus.state === 'checking' || updateStatus.state === 'downloading' || updateStatus.state === 'installing');
  // Background check failures stay in About. A check the user started, or any install
  // (Settings or the tray, which opens Settings when it fails), shows its failure until the next update step.
  let awaitingOutcome = false;
  let updateFailure = $state('');
  $effect(() => {
    const stop = watchUpdates(next => {
      const requested = awaitingOutcome || untrack(() => updateStatus.state === 'installing' || Boolean(updateFailure));
      updateStatus = next;
      if (['idle', 'ready', 'failed'].includes(next.state)) awaitingOutcome = false;
      updateFailure = next.state === 'failed' && requested ? next.message : '';
    }).catch(errors.report);
    return () => { stop.then(unlisten => unlisten?.()); };
  });
  // Color rows in Keybindings show each swatch's current color and name.
  let swatches = $derived(Object.fromEntries(COLOR_SHORTCUTS.map(choice => [choice.command,
    choice.slot !== undefined ? { background: swatchColor(preferences.swatches, choice.slot), name: swatchName(preferences.swatches, choice.slot) }
      : { background: choice.colorMode === 'rainbow' ? rainbowPreview(preferences.rainbowColors) : shiftingPreview(preferences.cycleColors), name: t(choice.name) }])));
  const run = (name: Action) => action(name).catch(errors.report);
  $effect(() => {
    if (!windowVisible) return;
    notice = ''; errors.clear();
    // Open where an unavailable shortcut can be replaced.
    if (untrack(() => session.unavailableShortcuts.length)) { tab = 'keybindings'; clearFilters(); }
  });
  $effect(() => () => clearTimeout(noticeTimer));

  const fold = (text: string) => text.normalize('NFKD').replace(/\p{M}/gu, '').toLocaleLowerCase(locale());
  function matchesBinding(binding: { label: string; group: string; shortcut: string; fallback: string; description?: string }) {
    if (onlyCustomized && sameShortcut(binding.shortcut, binding.fallback, platformMac)) return false;
    const terms = fold(shortcutSearch).trim().split(/\s+/);
    const text = fold(`${binding.label} ${binding.group} ${binding.description ?? ''} ${binding.shortcut ? shortcutLabel(binding.shortcut) : t('not set')}`);
    return terms.every(term => text.includes(term));
  }
  function clearFilters() { shortcutSearch = ''; onlyCustomized = false; }
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
  /** Every recorder releases the global shortcuts, so pressing one while recording reaches the "already used" check instead of the OS. */
  function record(recording: boolean, text: string) {
    hint = recording ? text : '';
    suspendGlobalShortcuts(recording).catch(errors.report);
  }
  function setGlobal(command: GlobalCommand, next: string) {
    const result = rebindGlobal(preferences, command, next);
    if (typeof result === 'string') errors.report(t(ALREADY_USED[result], { shortcut: shortcutLabel(next) }));
    else if (result) update({ shortcut: result.shortcut, captureShortcut: result.captureShortcut, keybindings: result.keybindings }, result.moved.length ? t('Unbound {commands} to make room.', { commands: list(result.moved.map(name => t(name))) }) : next ? t('Keybinding saved.') : '');
  }
  function setBinding(id: string, next: string) {
    const result = rebindCommand(preferences, id, next);
    if (typeof result === 'string') errors.report(t(ALREADY_USED[result], { shortcut: shortcutLabel(next) }));
    else if (result) update({ keybindings: result.keybindings }, result.moved.length ? t('Moved {shortcut} from {commands}.', { shortcut: shortcutLabel(next), commands: list(result.moved.map(name => t(name))) }) : '');
  }
  /** Native webviews don't open links themselves; Rust opens the fixed URL behind each action. */
  function openLink(event: MouseEvent, name: Action) {
    if (!native) return;
    event.preventDefault();
    run(name);
  }
  function updateClick() {
    if (updating || updateStatus.state === 'disabled') return;
    errors.clear();
    awaitingOutcome = true;
    (updateStatus.state === 'ready' ? installUpdate() : checkForUpdates()).catch(error => { awaitingOutcome = false; errors.report(error); });
  }
  async function toggleAutostart() {
    if (autostart === null) return;
    switchingAutostart = true; errors.clear();
    try { autostart = await setAutostart(!autostart); }
    catch (e) { errors.report(e); }
    finally { switchingAutostart = false; }
  }
  const fadeNote = (seconds: number) => seconds === 0 ? t('Marks stay until you return to work.') : t('Marks fade {duration} after you draw them.', { duration: number(seconds, { style: 'unit', unit: 'second', unitDisplay: 'long' }) });
</script>

<main class="settings-window">
  <section class="settings" aria-label={t('Glassboard settings')}>
    <header>
      <span class="brand"><Logo size={18}/>Glassboard</span>
      <div class="header-actions">
        <a class="icon-link" href={REPOSITORY_URL} target="_blank" rel="noreferrer" title={t('Glassboard on GitHub')} aria-label={t('Glassboard on GitHub')}
          onclick={event => openLink(event, 'open-github')}><GitHubIcon/></a>
        <button class="icon-link" title={t('Close')} aria-label={t('Close settings')} onclick={() => run('close-settings')}><X size={15}/></button>
      </div>
    </header>

    <div class="tabs" role="tablist" aria-label={t('Settings sections')}>
      {#each TABS as item}
        <button role="tab" id={`tab-${item.id}`} aria-selected={tab === item.id} aria-controls="settings-panel" tabindex={tab === item.id ? 0 : -1}
          onclick={() => select(item.id)} onkeydown={tabKeydown}><item.icon size={13} strokeWidth={2}/>{t(item.label)}</button>
      {/each}
    </div>

    <div class="panel" id="settings-panel" role="tabpanel" aria-labelledby={`tab-${tab}`} bind:this={panel}>
      {#if tab === 'general'}
        <section class="group" aria-labelledby="language-title">
          <h2 id="language-title" class="language-title"><Languages size={12} strokeWidth={2.2} aria-hidden="true"/>{t('Language')}{#if locale().split('-')[0] !== 'en'}<small lang="en">Language</small>{/if}</h2>
          <LanguagePicker language={preferences.language} onselect={language => update({ language })}/>
        </section>

        <section class="group" aria-labelledby="drawing-title">
          <h2 id="drawing-title">{t('Drawing')}</h2>
          <div class="drawing-setting">
            <h3 id="position-title">{t('Toolbar position')}</h3>
            <div class="positions" role="group" aria-labelledby="position-title">
              {#each POSITIONS as position}
                <button class="position" class:chosen={preferences.toolbarPosition === position.id} aria-pressed={preferences.toolbarPosition === position.id}
                  disabled={busy} onclick={() => update({ toolbarPosition: position.id })}>
                  <span class={`screen ${position.id}`} aria-hidden="true"><span></span></span>{t(position.label)}
                </button>
              {/each}
            </div>
          </div>
          <div class="drawing-setting">
            <h3 id="fade-title">{t('Auto-fade')}</h3>
            <div class="segmented" role="group" aria-labelledby="fade-title">
              {#each AUTO_FADE_OPTIONS as seconds}
                <button class:chosen={fade === seconds} aria-pressed={fade === seconds} disabled={busy} onclick={() => update({ autoFadeSeconds: seconds })}>{seconds ? number(seconds, { style: 'unit', unit: 'second', unitDisplay: 'narrow' }) : t('Off')}</button>
              {/each}
            </div>
            <p class="note">{fadeNote(fade)}</p>
          </div>
        </section>

        <section class="group" aria-labelledby="startup-title">
          <h2 id="startup-title">{t('Startup')}</h2>
          <div class="list">
            <div class="row">
              <span class="row-label" id="autostart-label">{t('Open at login')}<small>{platformMac ? t('Glassboard waits in the menu bar until you need it') : t('Glassboard waits in the tray until you need it')}</small></span>
              <button class="switch" role="switch" aria-checked={autostart === true} aria-labelledby="autostart-label"
                disabled={autostart === null || switchingAutostart} onclick={toggleAutostart}><span></span></button>
            </div>
          </div>
        </section>

        <section class="group" aria-labelledby="help-title">
          <h2 id="help-title">{t('Help')}</h2>
          <div class="list">
            <div class="row">
              <span class="row-label">{t('Tutorial')}<small>{t('A quick walkthrough of the basics')}</small></span>
              <button class="button" onclick={() => run('replay-tutorial')}>{t('Replay')}</button>
            </div>
            <div class="row">
              <span class="row-label">{t('Feedback')}<small>{t('Found a bug or have an idea? Open an issue.')}</small></span>
              <a class="button" href={`${REPOSITORY_URL}/issues/new`} target="_blank" rel="noreferrer" title={t('Opens GitHub Issues in your browser')}
                onclick={event => openLink(event, 'report-issue')}>{t('Report')}</a>
            </div>
          </div>
        </section>

        <section class="group" aria-labelledby="about-title">
          <h2 id="about-title">{t('About')}</h2>
          <div class="list">
            <div class="row update-row">
              <span class="row-label">Glassboard v{__APP_VERSION__}<small>{updateDetail(updateStatus)}</small></span>
              {#if updateStatus.state !== 'disabled'}
                <button class="button update" class:ready={updateStatus.state === 'ready'} aria-busy={updating} disabled={updating}
                  title={updateDetail(updateStatus)} aria-label={updateLabel(updateStatus)} onclick={updateClick}>
                  {#if updateStatus.state === 'ready'}<CircleArrowUp size={15}/>
                  {:else if updateStatus.state === 'downloading' && updateStatus.percent !== null}
                    <svg class="progress" width="15" height="15" viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6.5" pathLength="100"/><circle class="done" cx="8" cy="8" r="6.5" pathLength="100" stroke-dasharray={`${updateStatus.percent} 100`}/></svg>
                  {:else if updateStatus.state === 'downloading' || updateStatus.state === 'installing'}<span class="spin"><LoaderCircle size={15}/></span>
                  {:else}<span class:spin={updateStatus.state === 'checking'}><RefreshCw size={14}/></span>{/if}
                  <span>{updateLabel(updateStatus)}</span>
                </button>
              {/if}
            </div>
          </div>
        </section>
      {:else if tab === 'colors'}
        <ColorSettings {preferences} onsave={update}/>
      {:else}
        <div class="intro">
          <p>{t('Click a row, then press new keys.')}</p>
          <button class="text-button reset-all" disabled={busy || !customized} onclick={() => update({ shortcut: DEFAULT_SHORTCUT, captureShortcut: DEFAULT_CAPTURE_SHORTCUT, keybindings: {} }, t('All keybindings reset.'))}>{t('Reset all')}<RotateCcw size={12}/></button>
        </div>

        <div class="shortcut-filters">
          <label class="shortcut-search">
            <Search size={14} aria-hidden="true"/>
            <input type="search" aria-label={t('Search keybindings')} placeholder={t('Search keybindings')} bind:value={shortcutSearch}/>
          </label>
          <div class="customized-filter">
            <span id="customized-label">{t('Customized only')}</span>
            <button class="switch" role="switch" aria-checked={onlyCustomized} aria-labelledby="customized-label"
              onclick={() => onlyCustomized = !onlyCustomized}><span></span></button>
          </div>
        </div>

        {#if visibleGlobals.length}
        <section class="group" aria-labelledby="keys-global">
          <h2 id="keys-global">{t('Global')}</h2>
          <div class="list">
            {#each visibleGlobals as binding (binding.command)}
            <div class="row binding">
              <ShortcutRecorder label={binding.label} shortcut={binding.shortcut} fallback={binding.fallback} physical clearable={binding.command === 'capture'} active={windowVisible} disabled={busy}
                onchange={next => setGlobal(binding.command, next)} {errors} onrecord={recording => record(recording, binding.command === 'toggle' ? globalHint() : captureHint())}>
                <span class="row-icon">{#if binding.command === 'toggle'}<Logo size={14}/>{:else}<Scan size={15} strokeWidth={1.9}/>{/if}</span>
                <span class="row-label">{binding.label}</span>
              </ShortcutRecorder>
            </div>
            {/each}
          </div>
          {#if session.unavailableShortcuts.includes('toggle')}<p class="note unavailable" role="alert">{t('Another app is using {shortcut}, so it can’t turn on drawing. Record a different keybinding.', { shortcut: shortcutLabel(preferences.shortcut) })}</p>{/if}
          {#if session.unavailableShortcuts.includes('capture')}<p class="note unavailable" role="alert">{t('Another app is using {shortcut}, so it can’t take screenshots. Record a different keybinding.', { shortcut: shortcutLabel(preferences.captureShortcut) })}</p>{/if}
        </section>
        {/if}

        {#each visibleGroups as group (group.name)}
          <section class="group" aria-labelledby={`keys-${group.name}`}>
            <h2 id={`keys-${group.name}`}>{t(group.name)}</h2>
            <div class="list">
              {#each group.bindings as binding (binding.id)}
                {@const Icon = ICONS[binding.id]}
                {@const swatch = swatches[binding.id]}
                <div class="row binding">
                  <ShortcutRecorder label={binding.label} shortcut={binding.shortcut} fallback={binding.fallback} clearable active={windowVisible} disabled={busy}
                    onchange={next => setBinding(binding.id, next)} {errors} onrecord={recording => record(recording, localHint())}>
                    <span class="row-icon">{#if swatch}<span class="swatch" style:background={swatch.background}></span>{:else if Icon}<Icon size={15} strokeWidth={1.9}/>{/if}</span>
                    <span class="row-label">{binding.label}{#if binding.description}<small>{binding.description}</small>{/if}</span>
                  </ShortcutRecorder>
                </div>
              {/each}
            </div>
          </section>
        {/each}
        {#if !visibleGlobals.length && !visibleGroups.length}
          <div class="empty-shortcuts" role="status">
            <p>{shortcutSearch.trim() ? t('No keybindings match your search.') : t('No customized keybindings.')}</p>
            <button class="button" onclick={clearFilters}>{t('Show all keybindings')}</button>
          </div>
        {/if}
      {/if}
    </div>

    <div class="status" aria-live="polite">
      {#if errors.message}<p class="error" role="alert">{errors.message}<button class="dismiss" aria-label={t('Dismiss error')} onclick={errors.dismiss}><X size={12}/></button></p>
      {:else if hint}<p>{hint}</p>
      {:else if updateFailure}<p>{updateFailure}</p>
      {:else if notice}<p>{notice}</p>{/if}
    </div>

    <footer>
      {#if usage}<span class="usage" title={t('Memory and CPU used by Glassboard and its windows')}>{usageLabel(usage)}</span>{/if}
      <button onclick={() => run('quit')}><Power size={13}/>{t('Quit')}</button>
    </footer>
  </section>
</main>

<style>
  .settings-window { color-scheme: light dark; padding: 8px; width: 100%; height: 100vh; }
  .settings { --keycap: #ffffff; --raised: #ffffff; --reset: #d33d3d; display: flex; flex-direction: column; height: 100%; overflow: hidden; border: 1px solid var(--gb-border); border-radius: 14px; background: var(--gb-surface); box-shadow: 0 2px 7px var(--gb-shadow); }
  @media (prefers-color-scheme: dark) { .settings { --keycap: #3a3d44; --raised: #3d4048; --reset: #ff6b6b; } }

  header { display: flex; align-items: center; justify-content: space-between; padding: 14px 12px 0 16px; }
  .brand { display: flex; align-items: center; gap: 7px; font-size: 14px; font-weight: 600; letter-spacing: -.2px; color: var(--gb-strong-text); }
  .header-actions { display: flex; gap: 2px; }
  .icon-link { display: grid; place-items: center; width: 26px; height: 26px; border-radius: 7px; color: var(--gb-muted); transition: background .12s, color .12s; }
  .icon-link:hover { background: var(--gb-hover); color: var(--gb-strong-text); }
  .icon-link:focus-visible { outline: 2px solid var(--gb-focus-ring); outline-offset: 2px; }
  .update.ready { color: var(--gb-focus-ring); }
  .button.update { gap: 7px; min-height: 28px; padding-block: 5px; }
  .update:disabled { opacity: .65; }
  .row.update-row { flex-direction: column; align-items: flex-start; gap: 10px; padding: 12px; }
  .update-row small { line-height: 1.4; margin-top: 4px; }
  .update span.spin { display: grid; }
  .spin { animation: spin 1s linear infinite; }
  @keyframes spin { to { rotate: 360deg; } }
  @media (prefers-reduced-motion: reduce) { .spin { animation: none; opacity: .5; } }
  .progress { rotate: -90deg; }
  .progress circle { fill: none; stroke: color-mix(in srgb, currentColor 25%, transparent); stroke-width: 2; }
  .progress circle.done { stroke: var(--gb-focus-ring); stroke-linecap: round; transition: stroke-dasharray .2s; }

  .tabs, .segmented { display: flex; gap: 2px; padding: 3px; border-radius: 9px; background: color-mix(in srgb, var(--gb-text) 6%, transparent); }
  .tabs { margin: 14px 14px 0; }
  .tabs button, .segmented button { flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px; min-height: 26px; padding-block: 3px; line-height: 1.2; overflow-wrap: break-word; border-radius: 6px; font-size: 12px; font-weight: 500; color: var(--gb-secondary-text); transition: background .12s, color .12s, box-shadow .12s, transform 160ms var(--gb-ease-out); }
  .tabs button :global(svg) { flex-shrink: 0; }
  .tabs button:hover, .segmented button:hover:not(:disabled) { color: var(--gb-strong-text); }
  @media (prefers-reduced-motion: no-preference) { .tabs button:active, .segmented button:active:not(:disabled) { transform: scale(0.97); } }
  .tabs button[aria-selected="true"], .segmented button.chosen { background: var(--raised); color: var(--gb-strong-text); box-shadow: 0 0 0 1px var(--gb-selected-border), 0 1px 2px var(--gb-shadow); }
  .segmented button { font-size: 11.5px; font-variant-numeric: tabular-nums; }

  .panel { flex: 1; min-height: 0; overflow-y: auto; padding: 16px 14px 12px; scrollbar-width: thin; }
  /* Section styles are shared with tab components such as ColorSettings. */
  .panel :global(.group + .group) { margin-top: 20px; }
  .panel :global(h2) { margin: 0 0 8px 2px; font-size: 10px; font-weight: 600; letter-spacing: .8px; text-transform: uppercase; color: var(--gb-muted); }
  .language-title { display: flex; align-items: center; gap: 5px; }
  .language-title small { font-size: inherit; font-weight: 400; letter-spacing: 0; text-transform: none; }
  .note { margin: 8px 2px 0; font-size: 11px; color: var(--gb-muted); }

  .drawing-setting + .drawing-setting { margin-top: 16px; }
  .drawing-setting h3 { margin: 0 2px 8px; font-size: 12.5px; font-weight: 500; color: var(--gb-text); }
  .shortcut-filters { display: flex; flex-direction: column; gap: 10px; margin-bottom: 16px; }
  .shortcut-search { display: flex; align-items: center; gap: 8px; padding: 0 10px; min-height: 34px; border: 1px solid var(--gb-border); border-radius: 8px; background: var(--gb-input-surface); color: var(--gb-muted); }
  .shortcut-search:focus-within { outline: 2px solid var(--gb-focus-ring); outline-offset: 2px; }
  .shortcut-search input { min-width: 0; width: 100%; padding: 7px 0; border: 0; outline: none; background: transparent; color: var(--gb-text); font: inherit; font-size: 12px; }
  .shortcut-search input::placeholder { color: var(--gb-muted); }
  .customized-filter { display: flex; align-items: center; justify-content: space-between; gap: 10px; min-height: 40px; padding: 5px 6px 5px 12px; border: 1px solid var(--gb-divider); border-radius: 10px; background: var(--gb-subtle-surface); font-size: 12.5px; color: var(--gb-text); }
  .empty-shortcuts { padding: 24px 8px; text-align: center; color: var(--gb-muted); font-size: 12px; }
  .empty-shortcuts p { margin: 0 0 12px; }

  .positions { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .position { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 12px 6px 9px; border-radius: 10px; border: 1px solid var(--gb-divider); background: var(--gb-subtle-surface); font-size: 11.5px; color: var(--gb-secondary-text); transition: border-color .12s, background .12s, color .12s, transform 160ms var(--gb-ease-out); }
  .position:hover:not(:disabled) { background: var(--gb-hover); color: var(--gb-strong-text); }
  @media (prefers-reduced-motion: no-preference) { .position:active:not(:disabled) { transform: scale(0.97); } }
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
  .row-label { flex: 1; min-width: 0; overflow-wrap: anywhere; font-size: 12.5px; color: var(--gb-text); }
  .row.binding .row-label { min-width: 5em; }
  .row-label small { display: block; margin-top: 1px; font-size: 10.5px; color: var(--gb-muted); }
  .button { display: inline-flex; align-items: center; min-height: 26px; margin-right: 2px; padding: 4px 11px; color: inherit; text-decoration: none; border-radius: 7px; background: var(--raised); box-shadow: 0 0 0 1px var(--gb-border), 0 1px 1px var(--gb-shadow); font-size: 11.5px; font-weight: 500; transition: color .12s, box-shadow .12s; }
  .button:focus-visible { outline: 2px solid var(--gb-focus-ring); outline-offset: 2px; }
  .switch { position: relative; flex-shrink: 0; width: 34px; height: 20px; margin-right: 4px; padding: 0; border-radius: 10px; background: color-mix(in srgb, var(--gb-text) 18%, transparent); transition: background .16s; }
  .switch span { position: absolute; top: 2px; left: 2px; width: 16px; height: 16px; border-radius: 50%; background: #fff; box-shadow: 0 1px 2px #0000004d; transition: translate 160ms var(--gb-ease-out); }
  .switch[aria-checked="true"] { background: var(--gb-focus-ring); }
  .switch[aria-checked="true"] span { translate: 14px 0; }
  .switch:disabled { opacity: .5; }
  @media (prefers-reduced-motion: reduce) { .switch span { transition: none; } }
  .button:hover { color: var(--gb-strong-text); box-shadow: 0 0 0 1px var(--gb-selected-border), 0 1px 2px var(--gb-shadow); }

  .panel :global(.intro) { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin: 0 2px 16px; }
  .panel :global(.intro p) { margin: 0; font-size: 11.5px; color: var(--gb-muted); }
  .panel :global(.text-button) { display: flex; align-items: center; gap: 5px; flex-shrink: 0; padding: 4px 6px; margin-right: 5px; border-radius: 6px; font-size: 11.5px; font-weight: 500; color: var(--gb-secondary-text); transition: background .12s, color .12s; }
  .panel :global(.text-button:hover:not(:disabled)) { background: var(--gb-hover); color: var(--gb-strong-text); }
  .panel :global(.reset-all) { color: var(--reset); }
  .panel :global(.reset-all:hover:not(:disabled)) { background: color-mix(in srgb, var(--reset) 14%, transparent); color: var(--reset); }
  .panel :global(.reset-all:disabled) { color: var(--gb-muted); }

  .status { padding: 0 16px; font-size: 11px; line-height: 1.4; color: var(--gb-muted); }
  .status p { margin: 0 0 9px; }
  .status .error { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; color: var(--gb-error-text); overflow-wrap: anywhere; }
  .status .dismiss { display: grid; place-items: center; flex-shrink: 0; width: 18px; height: 18px; border-radius: 5px; color: var(--gb-muted); transition: background .12s, color .12s; }
  .status .dismiss:hover { background: var(--gb-hover); color: var(--gb-strong-text); }
  .note.unavailable { color: var(--gb-error-text); }
  footer { display: flex; align-items: center; justify-content: flex-end; padding: 10px 12px; border-top: 1px solid var(--gb-divider); }
  footer button { display: flex; align-items: center; gap: 6px; padding: 5px 6px; border-radius: 6px; font-size: 11.5px; color: var(--gb-secondary-text); transition: background .12s, color .12s; }
  footer button:hover { background: var(--gb-hover); color: var(--gb-strong-text); }
  .usage { margin-right: auto; padding-left: 6px; font-size: 11px; color: var(--gb-muted); font-variant-numeric: tabular-nums; }
</style>
