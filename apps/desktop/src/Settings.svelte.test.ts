// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import Settings from './Settings.svelte';
import { defaults, getAutostart, savePreferences, subscribe, type Session } from '@glassboard/ui/session';

let settings: ReturnType<typeof mount> | undefined;
let stop = () => {};
afterEach(async () => {
  if (settings) await unmount(settings);
  settings = undefined;
  stop();
  await savePreferences(structuredClone(defaults.preferences));
  document.body.replaceChildren();
});

async function setup(tab = 'keybindings') {
  let session = $state<Session>(structuredClone(defaults));
  stop = await subscribe(value => session = value);
  settings = mount(Settings, { target: document.body, props: { get session() { return session; }, error: '', onerror: vi.fn() } });
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

it('records a new binding after clicking anywhere on the row, even when the row is not focused', async () => {
  const session = await setup();
  row('Pen').click();
  row('Pen').blur();
  flushSync();
  expect(row('Pen').getAttribute('aria-label')).toBe('Pen: press a new shortcut');
  press('p', 'KeyP', { ctrlKey: true });
  await tick();
  expect(session().preferences.keybindings).toEqual({ 'tool-pen': 'CommandOrControl+KeyP' });
  expect(row('Pen').getAttribute('aria-label')).not.toContain('press a new shortcut');
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
  button('Reset #5856D6 to default').click();
  await tick();
  expect(session().preferences.swatches[4]).toBe('#f46b78');
  expect(session().preferences.color).toBe('#f46b78');
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
  for (let i = 0; i < 8; i++) { click('.chip-actions button'); await tick(); }
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
  expect(trigger.getAttribute('aria-label')).toBe('Theme: Glassboard');
  trigger.click();
  flushSync();
  await tick();
  const options = [...document.querySelectorAll<HTMLElement>('[role="option"][id^="theme-"]')];
  expect(options.map(option => option.textContent?.trim())).toEqual(['Glassboard', 'Spectrum', 'Neon', 'Sunset', 'Ocean', 'Pastel']);
  options.find(option => option.textContent?.includes('Neon'))!.click();
  await tick();
  const neon = ['#ff2bd6', '#ffe600', '#00ff9c', '#00e5ff', '#7c4dff'];
  expect(session().preferences).toMatchObject({ rainbowColors: neon, cycleColors: neon, swatches: ['#111111', '#eeeeee', '#00ff9c', '#ffe600', '#ff2bd6', '#00e5ff'], color: '#ff2bd6' });
  expect(document.querySelector('[role="listbox"][aria-label="Themes"]')).toBeNull();
  expect(trigger.getAttribute('aria-label')).toBe('Theme: Neon');

  // Escape closes the list without closing settings; a later edit makes the theme custom.
  trigger.click();
  flushSync();
  const list = document.querySelector<HTMLElement>('[role="listbox"][aria-label="Themes"]')!;
  const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
  const reachedWindow = vi.fn();
  window.addEventListener('keydown', reachedWindow);
  list.dispatchEvent(escape);
  window.removeEventListener('keydown', reachedWindow);
  flushSync();
  expect(reachedWindow).not.toHaveBeenCalled();
  expect(document.querySelector('[role="listbox"][aria-label="Themes"]')).toBeNull();
  // Black and White aren't part of a theme; the themed swatches are.
  document.querySelector<HTMLButtonElement>('button[aria-label="Use #000000"]')!.click();
  await tick();
  expect(trigger.getAttribute('aria-label')).toBe('Theme: Neon');
  document.querySelector<HTMLButtonElement>('button[aria-label^="#00FF9C,"]')!.click();
  flushSync();
  document.querySelector<HTMLButtonElement>('button[aria-label="Use #000000"]')!.click();
  await tick();
  expect(trigger.getAttribute('aria-label')).toBe('Theme: Custom');
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
