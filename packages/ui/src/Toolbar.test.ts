// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { fromStore, writable } from 'svelte/store';
import Toolbar from './Toolbar.svelte';
import { createErrors } from './lib/errors.svelte';
import { defaults, expandToolbar, toolbarPointer, type Session, type ToolbarPosition } from './lib/session';
import { shortcutLabel } from './lib/shortcuts';

vi.mock('./lib/session', async importOriginal => ({
  ...await importOriginal<typeof import('./lib/session')>(),
  expandToolbar: vi.fn().mockResolvedValue(undefined),
  toolbarPointer: vi.fn().mockResolvedValue(() => {}),
}));

let toolbar: ReturnType<typeof mount> | undefined;
beforeEach(() => { vi.useFakeTimers(); vi.clearAllMocks(); });
afterEach(async () => {
  if (toolbar) await unmount(toolbar);
  toolbar = undefined;
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  delete (document as Partial<Document>).elementFromPoint;
  document.body.replaceChildren();
});

async function setup(position: ToolbarPosition = 'bottom') {
  const store = writable<Session>({ ...structuredClone(defaults), mode: 'draw', tutorial: 'draw',
    preferences: { ...defaults.preferences, toolbarPosition: position } });
  const current = fromStore(store);
  toolbar = mount(Toolbar, { target: document.body, props: { get session() { return current.current; }, errors: createErrors(() => current.current), host: 'page' } });
  await tick();
  return store;
}

function button(label: string) { return document.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!; }
async function hover(node: HTMLElement) {
  node.dispatchEvent(new Event('pointerenter'));
  await vi.advanceTimersByTimeAsync(200);
  await tick();
}

it.each(['bottom', 'left', 'right'] as const)('shows tool and color shortcuts outside the %s scrolling toolbar', async position => {
  await setup(position);
  expect([...document.querySelectorAll('.tool-group button')].map(node => node.getAttribute('aria-label'))).toEqual(['Arrow', 'Pen', 'Square', 'Circle', 'Eraser', 'Text', 'Highlighter']);
  const pen = button('Pen');
  expect(pen.hasAttribute('title')).toBe(false);
  await hover(pen);
  let tip = document.querySelector('[role="tooltip"]')!;
  expect(tip.textContent).toContain('Pen');
  expect(tip.textContent).toContain(shortcutLabel('CommandOrControl+2'));
  expect(tip.closest('.toolbar')).toBeNull();
  expect(pen.getAttribute('aria-describedby')).toBe(tip.id);
  expect(expandToolbar).toHaveBeenLastCalledWith(true);

  pen.dispatchEvent(new Event('pointerleave'));
  await hover(button('Green'));
  tip = document.querySelector('[role="tooltip"]')!;
  expect(tip.textContent).toContain('Green');
  expect(tip.querySelector('kbd')?.textContent).toBe('5');
  expect(pen.hasAttribute('aria-describedby')).toBe(false);
  button('Green').dispatchEvent(new Event('pointerleave'));
  await vi.advanceTimersByTimeAsync(150);
  await tick();
  expect(document.querySelector('[role="tooltip"]')).toBeNull();
  expect(expandToolbar).toHaveBeenLastCalledWith(false);
});

it('supports keyboard focus and dismisses on click and window blur', async () => {
  await setup();
  const eraser = button('Eraser');
  eraser.focus();
  await tick();
  expect(document.querySelector('[role="tooltip"]')?.textContent).toContain(shortcutLabel('CommandOrControl+E'));
  eraser.click();
  flushSync();
  expect(document.querySelector('[role="tooltip"]')).toBeNull();
  await hover(button('Rainbow'));
  expect(document.querySelector('[role="tooltip"]')?.textContent).toContain('Changes as you draw');
  window.dispatchEvent(new Event('blur'));
  flushSync();
  expect(document.querySelector('[role="tooltip"]')).toBeNull();
});

it('does not show a stale hint after the pointer leaves before the delay', async () => {
  await setup();
  button('Arrow').dispatchEvent(new Event('pointerenter'));
  button('Arrow').dispatchEvent(new Event('pointerleave'));
  await vi.advanceTimersByTimeAsync(400);
  expect(document.querySelector('[role="tooltip"]')).toBeNull();
});

// WebKit may not send DOM pointer events until the toolbar is clicked. Native
// cursor samples must provide the same hover behavior without changing focus.
it('shows and updates unfocused tooltips from native cursor samples without a click', async () => {
  const store = await setup();
  store.update(session => ({ ...session, tutorial: null }));
  await vi.advanceTimersByTimeAsync(300);
  expect(document.querySelector('.toolbar-host')?.classList.contains('collapsed')).toBe(true);
  const focus = vi.spyOn(HTMLElement.prototype, 'focus');
  let hit: Element | null = button('Arrow').querySelector('svg');
  const hitTest = vi.fn(() => hit);
  Object.defineProperty(document, 'elementFromPoint', { configurable: true, value: hitTest });
  const sample = vi.mocked(toolbarPointer).mock.calls[0][0];
  await sample({ near: true, x: 50, y: 40 });
  // Repeated samples over the same button must not restart the delay.
  for (let i = 0; i < 4; i++) {
    await vi.advanceTimersByTimeAsync(80);
    await sample({ near: true, x: 50, y: 40 });
  }
  expect(hitTest).toHaveBeenCalledWith(50, 40);
  expect(document.querySelector('[role="tooltip"]')?.textContent).toContain('Arrow');
  expect(focus).not.toHaveBeenCalled();

  hit = button('Green');
  await sample({ near: true, x: 400, y: 40 });
  await tick();
  expect(document.querySelector('[role="tooltip"]')?.textContent).toContain('Green');
  hit = document.querySelector('[role="tooltip"]');
  await sample({ near: false, x: 400, y: 10 });
  await vi.advanceTimersByTimeAsync(200);
  expect(document.querySelector('[role="tooltip"]')?.textContent).toContain('Green');

  hit = button('Green');
  await sample({ near: true, x: 400, y: 40 });
  button('Green').click();
  await sample({ near: true, x: 400, y: 40 });
  await vi.advanceTimersByTimeAsync(300);
  expect(document.querySelector('[role="tooltip"]')).toBeNull();

  await sample(null);
  hit = button('Arrow');
  await sample({ near: true, x: 50, y: 40 });
  await vi.advanceTimersByTimeAsync(200);
  expect(document.querySelector('[role="tooltip"]')?.textContent).toContain('Arrow');
  await sample(null);
  await vi.advanceTimersByTimeAsync(500);
  expect(document.querySelector('[role="tooltip"]')).toBeNull();
  expect(document.querySelector('.toolbar-host')?.classList.contains('collapsed')).toBe(true);
});

it('stays open when pinned, even after the tutorial and with the cursor away', async () => {
  const store = writable<Session>({ ...structuredClone(defaults), mode: 'draw', tutorial: null });
  const current = fromStore(store);
  toolbar = mount(Toolbar, { target: document.body, props: { get session() { return current.current; }, errors: createErrors(() => current.current), host: 'page', pinned: true } });
  await tick();
  await vi.advanceTimersByTimeAsync(300);
  expect(document.querySelector('.toolbar-host')?.classList.contains('collapsed')).toBe(false);
  window.dispatchEvent(new MouseEvent('mouseout'));
  await vi.advanceTimersByTimeAsync(300);
  expect(document.querySelector('.toolbar-host')?.classList.contains('collapsed')).toBe(false);
});

it.each([['window', true], ['page', true], ['capture', false]] as const)('in a %s host, follows the native cursor and window size: %s', async (host, native) => {
  const store = writable<Session>({ ...structuredClone(defaults), mode: 'draw', tutorial: null });
  const current = fromStore(store);
  toolbar = mount(Toolbar, { target: document.body, props: { get session() { return current.current; }, errors: createErrors(() => current.current), host } });
  await tick();
  expect(vi.mocked(toolbarPointer).mock.calls.length > 0).toBe(native);
  expect(vi.mocked(expandToolbar).mock.calls.length > 0).toBe(native);
  expect(Boolean(document.querySelector('.fade-button'))).toBe(native);
});

it('reveals on a nearby pointer in a page, but leaves that to the native window otherwise', async () => {
  for (const host of ['page', 'window'] as const) {
    const store = writable<Session>({ ...structuredClone(defaults), mode: 'draw', tutorial: null });
    const current = fromStore(store);
    toolbar = mount(Toolbar, { target: document.body, props: { get session() { return current.current; }, errors: createErrors(() => current.current), host } });
    await tick();
    await vi.advanceTimersByTimeAsync(300);
    const toolbarHost = document.querySelector('.toolbar-host')!;
    expect(toolbarHost.classList.contains('collapsed')).toBe(true);
    window.dispatchEvent(new MouseEvent('pointermove', { clientX: 0, clientY: 0 }));
    await vi.advanceTimersByTimeAsync(0);
    expect(toolbarHost.classList.contains('collapsed')).toBe(host === 'window');
    await unmount(toolbar);
    toolbar = undefined;
    document.body.replaceChildren();
  }
});
