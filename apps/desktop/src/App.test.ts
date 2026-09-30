// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import App from './App.svelte';
import { action, defaults, savePreferences, subscribe, type Session } from '@glassboard/ui/session';

let app: ReturnType<typeof mount> | undefined;
let stop = () => {};

afterEach(async () => {
  if (app) await unmount(app);
  await action('hide');
  stop();
  document.body.replaceChildren();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function press(key: string, modifiers: KeyboardEventInit = {}, target: EventTarget = document.body) {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...modifiers });
  target.dispatchEvent(event);
  flushSync();
  return event;
}

it('routes tool and color shortcuts without consuming text entry, X, or work-mode keys', async () => {
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({ clearRect: vi.fn() } as unknown as CanvasRenderingContext2D);
  vi.stubGlobal('requestAnimationFrame', () => 1);
  vi.stubGlobal('cancelAnimationFrame', () => {});
  await action('dismiss-tutorial');
  await savePreferences({ ...defaults.preferences, tutorialCompleted: true });
  await action('show');
  let session: Session = structuredClone(defaults);
  stop = await subscribe(value => session = value);
  app = mount(App, { target: document.body });
  await tick();

  expect(press('s', { metaKey: true, shiftKey: true }).defaultPrevented).toBe(false);
  expect(session.capture).toBeNull();
  expect(document.querySelector('button[aria-label="Screenshot"]')).not.toBeNull();

  expect(press('1', { metaKey: true }).defaultPrevented).toBe(true);
  expect(session.preferences.tool).toBe('arrow');
  expect(press('8').defaultPrevented).toBe(true);
  expect(session.preferences).toMatchObject({ tool: 'arrow', colorMode: 'solid', color: '#669df0' });
  press('t', { metaKey: true });
  expect(session.preferences.tool).toBe('text');

  const input = document.createElement('textarea');
  document.body.append(input);
  expect(press('1', {}, input).defaultPrevented).toBe(false);
  for (const key of ['2', 'e', 't', 'h']) expect(press(key, { metaKey: true }, input).defaultPrevented).toBe(false);
  expect(session.preferences).toMatchObject({ tool: 'text', colorMode: 'solid', color: '#669df0' });
  expect(press('x').defaultPrevented).toBe(false);
  expect(press('e', { metaKey: true }).defaultPrevented).toBe(true);
  expect(session.preferences.tool).toBe('eraser');
  expect(session.preferences.color).toBe('#669df0');

  expect(press('h', { metaKey: true }).defaultPrevented).toBe(true);
  expect(session.preferences.tool).toBe('highlighter');
  expect(press('7', { metaKey: true }).defaultPrevented).toBe(false);

  await action('hide');
  await tick();
  expect(press('5').defaultPrevented).toBe(false);
  expect(session.preferences.color).toBe('#669df0');

  press('a', { metaKey: true, shiftKey: true });
  await tick();
  expect(press('s', { metaKey: true }).defaultPrevented).toBe(true);
  await tick(); await tick();
  expect(session.capture?.ready).toBe(true);
  expect(document.querySelector('.selection-surface')).not.toBeNull();
});
