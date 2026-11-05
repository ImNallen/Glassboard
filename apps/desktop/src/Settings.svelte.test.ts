// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import Settings from './Settings.svelte';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { action, createPreviewSession, defaults, savePreferences, subscribe, useSession, type Session } from '@glassboard/ui/session';
import { getAutostart } from './lib/autostart';
import { createErrors } from '@glassboard/ui/errors';
import { watchUpdates, type UpdateStatus } from './lib/updates';

vi.mock('./lib/updates', async original => {
  const updates = await original<typeof import('./lib/updates')>();
  return { ...updates, watchUpdates: vi.fn(updates.watchUpdates) };
});

let settings: ReturnType<typeof mount> | undefined;
let stop = () => {};
afterEach(async () => {
  vi.restoreAllMocks();
  clearMocks();
  if (settings) await unmount(settings);
  settings = undefined;
  stop();
  document.body.replaceChildren();
});

async function setup(tab = 'keybindings') {
  let session = $state<Session>(structuredClone(defaults));
  stop = await subscribe(value => session = value);
  settings = mount(Settings, { target: document.body, props: { get session() { return session; }, errors: createErrors(() => session) } });
  await tick();
  document.querySelector<HTMLButtonElement>(`#tab-${tab}`)!.click();
  flushSync();
  return () => session;
}
const row = (name: string) => document.querySelector<HTMLButtonElement>(`button[aria-label^="${name}:"]`)!;
// WebKit leaves focus on the body after clicking a button, so keys must not depend on the row having focus.
function press(key: string, code: string, init: KeyboardEventInit = {}) {
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key, code, bubbles: true, cancelable: true, ...init }));
  flushSync();
}

it('searches shortcut names, groups, descriptions, and assigned keys, and recovers from no results', async () => {
  await savePreferences(structuredClone(defaults.preferences));
  await setup();
  const search = document.querySelector<HTMLInputElement>('input[aria-label="Search keybindings"]')!;
  const filter = (value: string) => { search.value = value; search.dispatchEvent(new Event('input', { bubbles: true })); flushSync(); };
  filter('  PEN  ');
  expect(row('Pen')).not.toBeNull();
  expect(row('Undo')).toBeNull();
  expect(document.querySelectorAll('.group h2')).toHaveLength(1);
  filter('Global screenshot');
  expect(row('Take screenshot').getAttribute('aria-label')).toContain('Win+Ctrl+Shift+S');
  expect(row('Toggle Glassboard')).toBeNull();
  filter('screenshot editor');
  expect(row('Copy & close')).not.toBeNull();
  expect(row('Take screenshot')).toBeNull();
  filter('Ctrl+Z');
  expect(row('Undo')).not.toBeNull();
  expect(row('Redo')).toBeNull();
  filter('no matching command');
  expect(document.querySelectorAll('.group')).toHaveLength(0);
  expect(document.querySelector('[role="status"]')?.textContent).toContain('No keybindings match your search.');
  document.querySelector<HTMLButtonElement>('.empty-shortcuts button')!.click();
  flushSync();
  expect(search.value).toBe('');
  expect(row('Undo')).not.toBeNull();
  expect(row('Toggle Glassboard')).not.toBeNull();
});

it('filters customized local and global shortcuts, including cleared bindings, and updates after resetting', async () => {
  await savePreferences({ ...structuredClone(defaults.preferences), captureShortcut: '', keybindings: { 'tool-pen': 'CommandOrControl+KeyP', undo: '', redo: 'CommandOrControl+Shift+KeyZ' } });
  const session = await setup();
  const toggle = document.querySelector<HTMLButtonElement>('.customized-filter [role="switch"]')!;
  expect(toggle.getAttribute('aria-checked')).toBe('false');
  toggle.click();
  flushSync();
  expect(toggle.getAttribute('aria-checked')).toBe('true');
  expect(document.querySelectorAll('.recorder')).toHaveLength(3);
  expect(row('Take screenshot').getAttribute('aria-label')).toContain('not set');
  expect(row('Undo').getAttribute('aria-label')).toContain('not set');
  expect(row('Pen')).not.toBeNull();
  expect(row('Redo')).toBeNull();
  expect(row('Toggle Glassboard')).toBeNull();
  document.querySelector<HTMLButtonElement>('button[aria-label="Reset Pen to Ctrl+2"]')!.click();
  await tick();
  expect(session().preferences.keybindings['tool-pen']).toBeUndefined();
  expect(row('Pen')).toBeNull();
  document.querySelector<HTMLButtonElement>('.reset-all')!.click();
  await tick();
  expect(document.querySelector('[role="status"]')?.textContent).toContain('No customized keybindings.');
  expect(document.querySelector<HTMLButtonElement>('.reset-all')!.disabled).toBe(true);
  document.querySelector<HTMLButtonElement>('.empty-shortcuts button')!.click();
  flushSync();
  expect(toggle.getAttribute('aria-checked')).toBe('false');
  expect(row('Pen')).not.toBeNull();
});

it('records a new binding after clicking anywhere on the row, even when the row is not focused', async () => {
  const session = await setup();
  row('Pen').click();
  row('Pen').blur();
  flushSync();
  expect(row('Pen').getAttribute('aria-label')).toBe('Pen: press a new keybinding');
  press('p', 'KeyP', { ctrlKey: true });
  await tick();
  expect(session().preferences.keybindings).toEqual({ 'tool-pen': 'CommandOrControl+KeyP' });
  expect(row('Pen').getAttribute('aria-label')).not.toContain('press a new keybinding');
});

it('moves a taken shortcut, and stops recording on Escape or an outside click', async () => {
  const session = await setup();
  row('Pen').click();
  press('1', 'Digit1', { ctrlKey: true });
  await tick();
  expect(session().preferences.keybindings).toEqual({ 'tool-pen': 'CommandOrControl+Digit1', 'tool-arrow': '' });
  expect(document.querySelector('.status')?.textContent).toContain('from Arrow');

  row('Undo').click();
  press('Escape', 'Escape');
  expect(row('Undo').getAttribute('aria-label')).toBe('Undo: Ctrl+Z');
  row('Undo').click();
  document.body.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
  flushSync();
  expect(row('Undo').getAttribute('aria-label')).toBe('Undo: Ctrl+Z');
  press('u', 'KeyU', { ctrlKey: true });
  await tick();
  expect(session().preferences.keybindings.undo).toBeUndefined();
});

it('replaces a swatch with a preset, keeping a selected swatch selected, and resets it', async () => {
  await savePreferences({ ...structuredClone(defaults.preferences), colorMode: 'solid', color: '#f46b78' });
  const session = await setup('colors');
  const button = (label: string) => document.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
  document.querySelector<HTMLButtonElement>('button[aria-label^="Red,"]')!.click();
  flushSync();
  button('Use #5856D6').click();
  await tick();
  expect(session().preferences.swatches[4]).toBe('#5856d6');
  expect(session().preferences.color).toBe('#5856d6');
  expect(document.querySelector('#edit-title')?.textContent).toBe('#5856D6');
  const red = document.querySelector<HTMLButtonElement>('button[aria-label^="Red,"]')!;
  expect(red.textContent).toContain('Red');
  expect(red.getAttribute('aria-checked')).toBe('true');
  button('Reset #5856D6 to default').click();
  await tick();
  expect(session().preferences.swatches[4]).toBe('#f46b78');
  expect(session().preferences.color).toBe('#f46b78');
});

it('selects toolbar colors with arrow keys and keeps one keyboard stop', async () => {
  await setup('colors');
  const targets = [...document.querySelectorAll<HTMLButtonElement>('.slot')];
  targets[2].focus();
  targets[2].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }));
  flushSync();
  expect(document.activeElement).toBe(targets[3]);
  expect(targets[3].getAttribute('aria-checked')).toBe('true');
  expect(document.querySelector('#edit-title')?.textContent).toBe('White');
  expect(targets.filter(target => target.tabIndex === 0)).toEqual([targets[3]]);
  targets[3].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }));
  flushSync();
  expect(document.activeElement).toBe(targets[7]);
  expect(document.querySelector('#edit-title')?.textContent).toBe('Blue');
  targets[7].dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', bubbles: true, cancelable: true }));
  flushSync();
  expect(document.activeElement).toBe(targets[0]);
  expect(document.querySelector('#edit-title')?.textContent).toBe('Rainbow');
});

it('keeps invalid hex input visible without saving, then saves a corrected color', async () => {
  const session = await setup('colors');
  const disclosure = document.querySelector<HTMLDetailsElement>('.custom-color')!;
  expect(disclosure.open).toBe(false);
  disclosure.open = true;
  const input = document.querySelector<HTMLInputElement>('input[aria-label="Black hex code"]')!;
  const original = session().preferences.swatches[0];
  input.value = '#oops';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.dispatchEvent(new FocusEvent('blur'));
  await tick();
  expect(input.value).toBe('#oops');
  expect(input.getAttribute('aria-invalid')).toBe('true');
  expect(document.querySelector('.hex-error')?.textContent).toContain('3 or 6 hex digits');
  expect(session().preferences.swatches[0]).toBe(original);
  input.value = '#abc';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
  await tick();
  expect(session().preferences.swatches[0]).toBe('#aabbcc');
  expect(input.value).toBe('#AABBCC');
  expect(input.getAttribute('aria-invalid')).toBe('false');
  expect(document.querySelector('.hex-error')).toBeNull();
  expect(document.querySelector<HTMLButtonElement>('button[aria-label^="Black,"]')?.textContent).toContain('Black');
});

it('edits the Rainbow list: adding, reordering, removing, and reset', async () => {
  const session = await setup('colors');
  const click = (selector: string) => { document.querySelector<HTMLButtonElement>(selector)!.click(); flushSync(); };
  click('button[aria-label^="Rainbow,"]');
  expect(document.querySelectorAll('.chip')).toHaveLength(7);
  click('button[aria-label="Add a color"]');
  await tick();
  expect(session().preferences.rainbowColors).toHaveLength(8);
  expect(document.querySelector<HTMLButtonElement>('button[aria-label="Add a color"]')!.disabled).toBe(true);
  const added = session().preferences.rainbowColors[1];
  document.querySelector<HTMLButtonElement>('.chip.selected')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', altKey: true, bubbles: true }));
  await tick();
  expect(session().preferences.rainbowColors[0]).toBe(added);
  for (let i = 0; i < 8; i++) { click('.chip-actions button:not(.add)'); await tick(); }
  expect(session().preferences.rainbowColors).toHaveLength(2);
  expect(session().preferences.cycleColors).toHaveLength(7);
  click('button[aria-label="Reset Rainbow to default"]');
  await tick();
  expect(session().preferences.rainbowColors).toHaveLength(7);
});

it('applies a theme from the dropdown to Rainbow, Shifting, and keys 5–8, keeping Black, White, and the chosen swatch', async () => {
  await savePreferences({ ...structuredClone(defaults.preferences), swatches: ['#111111', '#eeeeee', '#4dcaa0', '#f2c85b', '#f46b78', '#669df0'], colorMode: 'solid', color: '#f46b78' });
  // jsdom has no layout; the dropdown keeps its highlighted theme scrolled into view.
  Element.prototype.scrollIntoView = vi.fn();
  const session = await setup('colors');
  const trigger = document.querySelector<HTMLButtonElement>('button[aria-haspopup="listbox"]')!;
  expect(trigger.getAttribute('aria-label')).toBe('Color palette: Glassboard');
  trigger.click();
  flushSync();
  await tick();
  const options = [...document.querySelectorAll<HTMLElement>('[role="option"][id^="theme-"]')];
  expect(options.map(option => option.textContent?.trim())).toEqual(['Glassboard', 'Spectrum', 'Neon', 'Sunset', 'Ocean', 'Pastel']);
  options.find(option => option.textContent?.includes('Neon'))!.click();
  await tick();
  const neon = ['#ff2bd6', '#ffe600', '#00ff9c', '#00e5ff', '#7c4dff'];
  expect(session().preferences).toMatchObject({ rainbowColors: neon, cycleColors: neon, swatches: ['#111111', '#eeeeee', '#00ff9c', '#ffe600', '#ff2bd6', '#00e5ff'], color: '#ff2bd6' });
  expect(document.querySelector('[role="listbox"][aria-label="Color palettes"]')).toBeNull();
  expect(trigger.getAttribute('aria-label')).toBe('Color palette: Neon');

  // Escape closes the list without closing settings; a later edit makes the theme custom.
  trigger.click();
  flushSync();
  const list = document.querySelector<HTMLElement>('[role="listbox"][aria-label="Color palettes"]')!;
  const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
  const reachedWindow = vi.fn();
  window.addEventListener('keydown', reachedWindow);
  list.dispatchEvent(escape);
  window.removeEventListener('keydown', reachedWindow);
  flushSync();
  expect(reachedWindow).not.toHaveBeenCalled();
  expect(document.querySelector('[role="listbox"][aria-label="Color palettes"]')).toBeNull();
  // Black and White aren't part of a theme; the themed swatches are.
  document.querySelector<HTMLButtonElement>('button[aria-label="Use #000000"]')!.click();
  await tick();
  expect(trigger.getAttribute('aria-label')).toBe('Color palette: Neon');
  document.querySelector<HTMLButtonElement>('button[aria-label^="Green,"]')!.click();
  flushSync();
  document.querySelector<HTMLButtonElement>('button[aria-label="Use #000000"]')!.click();
  await tick();
  expect(trigger.getAttribute('aria-label')).toBe('Color palette: Custom');
});

it('previews solid, Rainbow, and Shifting strokes using the edited colors', async () => {
  await savePreferences(structuredClone(defaults.preferences));
  const session = await setup('colors');
  const preview = () => document.querySelector<SVGSVGElement>('.stroke-preview svg')!;
  expect(preview().getAttribute('aria-label')).toBe('Black stroke preview');
  expect(preview().querySelector('path')?.getAttribute('stroke')).toBe('#000000');
  document.querySelector<HTMLButtonElement>('button[aria-label="Use #5856D6"]')!.click();
  await tick();
  expect(preview().querySelector('path')?.getAttribute('stroke')).toBe(session().preferences.swatches[0]);
  document.querySelector<HTMLButtonElement>('button[aria-label^="Rainbow,"]')!.click();
  flushSync();
  expect(preview().getAttribute('aria-label')).toBe('Rainbow stroke preview');
  expect(preview().querySelector('path')?.getAttribute('stroke')).toMatch(/^url\(#.+\)$/);
  document.querySelector<HTMLButtonElement>('button[aria-label="Use #5856D6"]')!.click();
  await tick();
  const rainbow = session().preferences.rainbowColors;
  expect([...preview().querySelectorAll('stop')].map(stop => stop.getAttribute('stop-color'))).toEqual([...rainbow, rainbow[0]]);
  document.querySelector<HTMLButtonElement>('button[aria-label^="Shifting,"]')!.click();
  flushSync();
  expect(preview().getAttribute('aria-label')).toBe('Shifting stroke preview');
  expect([...preview().querySelectorAll('path')].map(path => path.getAttribute('stroke'))).toEqual(session().preferences.cycleColors);
  document.querySelector<HTMLButtonElement>('button[aria-label="Add a color"]')!.click();
  await tick();
  expect([...preview().querySelectorAll('path')].map(path => path.getAttribute('stroke'))).toEqual(session().preferences.cycleColors);
});

it('reorders Rainbow colors by dragging a chip with the pointer', async () => {
  const session = await setup('colors');
  document.querySelector<HTMLButtonElement>('button[aria-label^="Rainbow,"]')!.click();
  flushSync();
  const chips = [...document.querySelectorAll<HTMLButtonElement>('.chip')];
  // jsdom has no layout: place chips 34px apart in a row, and accept pointer capture.
  chips.forEach((chip, i) => {
    chip.getBoundingClientRect = () => ({ left: i * 34, top: 0, width: 26, height: 26, right: i * 34 + 26, bottom: 26, x: i * 34, y: 0, toJSON() {} });
    chip.setPointerCapture = vi.fn();
  });
  const pointer = (type: string, x: number) => {
    const event = new MouseEvent(type, { clientX: x, clientY: 13, bubbles: true, button: 0 });
    Object.defineProperty(event, 'pointerId', { value: 1 });
    chips[0].dispatchEvent(event);
    flushSync();
  };
  const original = [...session().preferences.rainbowColors];
  pointer('pointerdown', 13);
  pointer('pointermove', 80);
  pointer('pointermove', 115);
  expect(document.querySelectorAll('.chip')[3].classList.contains('dragging')).toBe(true);
  pointer('pointerup', 115);
  chips[0].click();
  await tick();
  expect(session().preferences.rainbowColors).toEqual([original[1], original[2], original[3], original[0], ...original.slice(4)]);
  expect(document.querySelector('.chip.selected')?.getAttribute('data-index')).toBe('3');
});

it('turns opening at login on and off from the General tab', async () => {
  await setup('general');
  const toggle = () => document.querySelector<HTMLButtonElement>('button[role="switch"]')!;
  await vi.waitFor(() => expect(toggle().disabled).toBe(false));
  expect(toggle().getAttribute('aria-checked')).toBe('false');
  toggle().click();
  await vi.waitFor(() => expect(toggle().getAttribute('aria-checked')).toBe('true'));
  expect(await getAutostart()).toBe(true);
  toggle().click();
  await vi.waitFor(() => expect(toggle().getAttribute('aria-checked')).toBe('false'));
  expect(await getAutostart()).toBe(false);
});

it('checks for updates beside the version information without a message when nothing is new', async () => {
  await setup('general');
  const button = () => document.querySelector<HTMLButtonElement>('.update-row button[aria-label="Check for updates"]')!;
  expect(button().title).toBe('Glassboard checks GitHub for new versions once a day.');
  expect(button().textContent).toContain('Check for updates');
  expect(document.querySelector('.update-row')?.textContent).toContain('Glassboard v');
  button().click();
  await tick();
  expect(button().title).toBe('Glassboard checks GitHub for new versions once a day.');
  expect(document.querySelector('.status')!.textContent).toBe('');
});

it('shows update progress and restart actions beside the version, and disables repeated requests', async () => {
  let emit: (status: UpdateStatus) => void = () => {};
  vi.mocked(watchUpdates).mockImplementationOnce(async fn => { emit = fn; fn({ state: 'idle' }); return () => {}; });
  await setup('general');
  const button = () => document.querySelector<HTMLButtonElement>('.update-row button')!;
  emit({ state: 'checking' });
  flushSync();
  expect(button().disabled).toBe(true);
  expect(button().textContent).toContain('Checking…');
  expect(button().getAttribute('aria-busy')).toBe('true');
  emit({ state: 'downloading', version: '0.2.0', percent: 42 });
  flushSync();
  expect(button().disabled).toBe(true);
  expect(document.querySelector('.update-row')?.textContent).toContain('42%');
  emit({ state: 'ready', version: '0.2.0' });
  flushSync();
  expect(button().disabled).toBe(false);
  expect(button().textContent).toContain('Restart to install version 0.2.0');
  emit({ state: 'installing', version: '0.2.0' });
  flushSync();
  expect(button().disabled).toBe(true);
  emit({ state: 'failed', message: 'Could not install the update.' });
  flushSync();
  expect(button().disabled).toBe(false);
  expect(document.querySelector('.update-row')?.textContent).toContain('Could not install the update.');
  emit({ state: 'disabled' });
  flushSync();
  expect(button()).toBeNull();
  expect(document.querySelector('.update-row')?.textContent).toContain('Automatic updates are unavailable in this build.');
});

it('shows why an install started from the tray failed, but not a background check failure', async () => {
  let emit: (status: UpdateStatus) => void = () => {};
  vi.mocked(watchUpdates).mockImplementationOnce(async fn => { emit = fn; fn({ state: 'ready', version: '0.2.0' }); return () => {}; });
  await setup('general');
  const status = () => document.querySelector('.status')!.textContent;
  emit({ state: 'installing', version: '0.2.0' });
  emit({ state: 'failed', message: 'Move Glassboard to Applications to install updates.' });
  flushSync();
  expect(status()).toBe('Move Glassboard to Applications to install updates.');
  emit({ state: 'checking' });
  emit({ state: 'failed', message: "Couldn't reach GitHub to check for updates." });
  flushSync();
  expect(status()).toBe('');
});

it('opens on keybindings and explains each global shortcut another app holds', async () => {
  const session = $state<Session>({ ...structuredClone(defaults), unavailableShortcuts: ['capture'] });
  settings = mount(Settings, { target: document.body, props: { get session() { return session; }, errors: createErrors(() => session) } });
  await tick();
  const notes = () => [...document.querySelectorAll('.note.unavailable')].map(note => note.textContent);
  expect(document.querySelector('#tab-keybindings')?.getAttribute('aria-selected')).toBe('true');
  expect(notes()).toEqual(['Another app is using Win+Ctrl+Shift+S, so it can’t take screenshots. Record a different keybinding.']);
  session.unavailableShortcuts = ['toggle', 'capture'];
  flushSync();
  expect(notes()).toEqual([
    'Another app is using Ctrl+Shift+A, so it can’t turn on drawing. Record a different keybinding.',
    'Another app is using Win+Ctrl+Shift+S, so it can’t take screenshots. Record a different keybinding.',
  ]);
  session.unavailableShortcuts = [];
  flushSync();
  expect(notes()).toEqual([]);
});

it('records, refuses, clears, and resets the global screenshot shortcut', async () => {
  const session = await setup();
  const status = () => document.querySelector('.status')?.textContent;
  expect(row('Take screenshot').getAttribute('aria-label')).toBe('Take screenshot: Win+Ctrl+Shift+S');
  row('Take screenshot').click();
  press('A', 'KeyA', { ctrlKey: true, shiftKey: true });
  await tick();
  expect(status()).toBe('Ctrl+Shift+A is already used to toggle Glassboard.');
  expect(session().preferences.captureShortcut).toBe('Super+Control+Shift+KeyS');

  row('Take screenshot').click();
  press('x', 'KeyX', { ctrlKey: true, altKey: true });
  await tick();
  expect(session().preferences.captureShortcut).toBe('CommandOrControl+Alt+KeyX');
  row('Toggle Glassboard').click();
  press('x', 'KeyX', { ctrlKey: true, altKey: true });
  await tick();
  expect(status()).toBe('Ctrl+Alt+X is already used to take a screenshot.');
  row('Undo').click();
  press('x', 'KeyX', { ctrlKey: true, altKey: true });
  await tick();
  expect(status()).toBe('Ctrl+Alt+X is already used to take a screenshot.');

  row('Take screenshot').click();
  press('Backspace', 'Backspace');
  await tick();
  expect(session().preferences.captureShortcut).toBe('');
  expect(row('Take screenshot').getAttribute('aria-label')).toBe('Take screenshot: not set');
  document.querySelector<HTMLButtonElement>('.reset-all')!.click();
  await tick();
  expect(session().preferences.captureShortcut).toBe('Super+Control+Shift+KeyS');
  expect(document.querySelector<HTMLButtonElement>('.reset-all')!.disabled).toBe(true);
});

it('releases the global shortcuts while any binding records, and holds them again when recording stops', async () => {
  const requests: unknown[] = [];
  mockIPC((command, payload) => {
    if (command === 'suspend_global_shortcuts') requests.push(payload);
    if (command === 'get_autostart') return false;
  });
  vi.mocked(watchUpdates).mockImplementationOnce(async fn => { fn({ state: 'idle' }); return () => {}; });
  useSession(createPreviewSession(), { native: true });
  await action('settings');
  const session = await setup();
  row('Toggle Glassboard').click();
  flushSync();
  expect(requests).toEqual([{ suspended: true }]);
  press('Escape', 'Escape');
  expect(requests).toEqual([{ suspended: true }, { suspended: false }]);

  row('Undo').click();
  flushSync();
  press('u', 'KeyU', { ctrlKey: true });
  await tick();
  expect(session().preferences.keybindings.undo).toBe('CommandOrControl+KeyU');
  expect(requests).toEqual([{ suspended: true }, { suspended: false }, { suspended: true }, { suspended: false }]);

  row('Redo').click();
  flushSync();
  expect(requests).toHaveLength(5);
  await action('close-settings');
  flushSync();
  expect(requests.at(-1)).toEqual({ suspended: false });
  expect(requests).toHaveLength(6);
});

it('dismisses a backend error from the status line', async () => {
  useSession(createPreviewSession({ error: 'Some settings could not be read.' }));
  await setup();
  const status = () => document.querySelector('.status [role="alert"]');
  expect(status()?.textContent).toContain('Some settings could not be read.');
  document.querySelector<HTMLButtonElement>('button[aria-label="Dismiss error"]')!.click();
  await vi.waitFor(() => expect(status()).toBeNull());
});
