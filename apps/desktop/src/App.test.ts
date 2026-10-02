// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import App from './App.svelte';
import { action, createPreviewSession, defaults, subscribe, useSession, type Session } from '@glassboard/ui/session';
import { parseSurface, windowParts, type Part, type Surface } from './lib/surface';

let app: ReturnType<typeof mount> | undefined;
let stop = () => {};

beforeEach(() => {
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({ clearRect: vi.fn(), putImageData: vi.fn() } as unknown as CanvasRenderingContext2D);
  vi.stubGlobal('requestAnimationFrame', () => 1);
  vi.stubGlobal('cancelAnimationFrame', () => {});
});
afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  stop();
  document.body.replaceChildren();
  history.replaceState(null, '', '/');
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

async function open(surface: Surface | null, initial: Partial<Session> = {}, native = false) {
  history.replaceState(null, '', surface ? `/?surface=${surface}` : '/');
  useSession(createPreviewSession({ tutorial: 'welcome', ...initial }), { native });
  let session: Session = structuredClone(defaults);
  stop = await subscribe(value => session = value);
  app = mount(App, { target: document.body });
  await tick(); await tick();
  return () => session;
}

function press(key: string, modifiers: KeyboardEventInit = {}) {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...modifiers });
  document.body.dispatchEvent(event);
  flushSync();
  return event;
}

const MARKERS: Record<Part, string> = { backdrop: '.preview-background', overlay: 'canvas[aria-label^="Screen annotation"]', toolbar: '.toolbar-host', settings: '#tab-general', tutorial: 'aside[aria-label="Glassboard tutorial"]', capture: '.capture-editor' };
// The capture editor embeds its own overlay and toolbar, so only parts outside it count as the window's.
const mounted = () => (Object.keys(MARKERS) as Part[]).filter(part => [...document.querySelectorAll(MARKERS[part])].some(element => part === 'capture' || !element.closest('.capture-editor')));

const pending = { id: 1, ready: false }, ready = { id: 1, ready: true };
it.each<[Surface | null, boolean, Session['capture'], Part[]]>([
  ['overlay', true, null, ['overlay']],
  ['overlay', true, ready, []],
  ['toolbar', true, null, ['toolbar']],
  ['toolbar', true, pending, []],
  ['tutorial', true, null, ['tutorial']],
  ['tutorial', true, ready, []],
  ['settings', true, null, ['settings']],
  ['settings', true, ready, ['settings']],
  ['capture', true, null, []],
  ['capture', true, pending, []],
  ['capture', true, ready, ['capture']],
  [null, true, null, []],
  [null, false, null, ['backdrop', 'overlay', 'toolbar', 'tutorial']],
  [null, false, pending, []],
  [null, false, ready, ['capture']],
  ['settings', false, null, ['settings']],
  ['settings', false, ready, ['settings', 'capture']],
  ['toolbar', false, null, ['toolbar']],
  ['toolbar', false, ready, ['capture']],
])('on surface %s with native %s and capture %o mounts %o', async (surface, native, capture, parts) => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
  expect(windowParts(surface, native, capture)).toEqual(parts);
  await open(surface, { capture }, native);
  expect(mounted()).toEqual(parts);
});

it('reads only the surfaces Rust opens windows for', () => {
  expect(parseSurface('?surface=settings')).toBe('settings');
  expect(parseSurface('?surface=overlay-0')).toBeNull();
  expect(parseSurface('')).toBeNull();
});

it('closes the settings window on Escape without leaving drawing or routing other keys', async () => {
  const session = await open('settings', { mode: 'draw', settingsOpen: true });
  expect(press('t', { metaKey: true }).defaultPrevented).toBe(false);
  expect(press('Escape').defaultPrevented).toBe(true);
  await tick();
  expect(session()).toMatchObject({ settingsOpen: false, mode: 'draw', preferences: { tool: defaults.preferences.tool } });
});

it('dismisses the tutorial on Escape, or returns to work while drawing', async () => {
  const session = await open('tutorial');
  expect(press('Escape').defaultPrevented).toBe(true);
  await tick();
  expect(session()).toMatchObject({ tutorial: null, mode: 'hidden' });

  await action('replay-tutorial');
  await action('tutorial-start');
  press('Escape');
  await tick();
  expect(session()).toMatchObject({ tutorial: 'welcome', mode: 'hidden' });
});

it('routes drawing shortcuts to the session and opens the capture editor from Save', async () => {
  const session = await open(null, { tutorial: null, mode: 'draw' });
  expect(press('t', { metaKey: true }).defaultPrevented).toBe(true);
  expect(session().preferences.tool).toBe('text');
  expect(press('s', { metaKey: true }).defaultPrevented).toBe(true);
  await tick(); await tick();
  expect(session().capture?.ready).toBe(true);
  expect(mounted()).toEqual(['capture']);
  expect(document.querySelector('.selection-surface')).not.toBeNull();
  await new Promise(resolve => setTimeout(resolve));
  expect(document.querySelector('.capture-editor [role="alert"]')).toBeNull();
});
