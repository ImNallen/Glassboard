// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { fromStore, writable } from 'svelte/store';
import Capture from './Capture.svelte';
import { action, defaults, savePreferences, subscribe, type Session } from '@glassboard/ui/session';
import { annotatedCapture } from '@glassboard/ui/capture';
import { copyCaptureImage, getCaptureImage } from './lib/capture';

vi.mock('./lib/capture', () => ({ getCaptureImage: vi.fn().mockResolvedValue({ width: 2560, height: 1600, data: new Uint8ClampedArray(0) }), copyCaptureImage: vi.fn() }));
vi.mock('@glassboard/ui/capture', async original => ({ ...await original<typeof import('@glassboard/ui/capture')>(), annotatedCapture: vi.fn().mockResolvedValue(new Blob(['image'], { type: 'image/png' })) }));
vi.mock('@glassboard/ui/drawing', async original => ({ ...await original<typeof import('@glassboard/ui/drawing')>(), render: vi.fn() }));

let component: ReturnType<typeof mount> | undefined;
let stop = () => {};
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(copyCaptureImage).mockResolvedValue(undefined);
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({ putImageData: vi.fn(), clearRect: vi.fn(), save: vi.fn(), restore: vi.fn(), measureText: () => ({ width: 120 }) } as unknown as CanvasRenderingContext2D);
  vi.stubGlobal('requestAnimationFrame', () => 1);
  vi.stubGlobal('cancelAnimationFrame', () => {});
});
afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  stop();
  await action('cancel-capture');
  document.body.replaceChildren();
  vi.restoreAllMocks(); vi.unstubAllGlobals();
});

function select(from: [number, number], to: [number, number]) {
  const surface = document.querySelector('.selection-surface') as HTMLElement;
  surface.setPointerCapture = vi.fn();
  for (const [type, [x, y]] of [['pointerdown', from], ['pointerup', to]] as const) {
    surface.dispatchEvent(Object.assign(new MouseEvent(type, { bubbles: true, button: 0, clientX: x, clientY: y }), { pointerId: 1 }));
    flushSync();
  }
}

async function setup() {
  await action('dismiss-tutorial');
  await savePreferences({ ...defaults.preferences, toolbarPosition: 'left', autoFadeSeconds: 5 });
  await action('capture');
  const store = writable<Session>(structuredClone(defaults));
  let session = structuredClone(defaults);
  stop = await subscribe(value => { session = value; store.set(value); });
  const current = fromStore(store);
  component = mount(Capture, { target: document.body, props: { get session() { return current.current; }, onerror: vi.fn() } });
  await tick(); await tick();
  flushSync();
  select([100, 100], [600, 400]);
  return { state: () => session };
}

it('keeps failed copies open and closes only after clipboard success', async () => {
  const app = await setup();
  vi.mocked(copyCaptureImage).mockRejectedValueOnce(new Error('Clipboard busy'));
  (document.querySelector('button.copy') as HTMLButtonElement).click();
  await tick(); await tick();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('Clipboard busy');
  expect(app.state().capture).not.toBeNull();
  let finish!: () => void;
  vi.mocked(copyCaptureImage).mockImplementationOnce(() => new Promise(resolve => finish = resolve));
  (document.querySelector('button.copy') as HTMLButtonElement).click();
  await tick();
  expect(app.state().capture).not.toBeNull();
  expect((document.querySelector('button.copy') as HTMLButtonElement).disabled).toBe(true);
  finish(); await tick(); await tick();
  expect(app.state().capture).toBeNull();
});

it('uses permanent capture drawings without changing live fade or dock preferences', async () => {
  const app = await setup();
  expect(document.querySelector('.toolbar-host')?.classList.contains('vertical')).toBe(true);
  expect(document.querySelector('.toolbar-host')?.classList.contains('left')).toBe(true);
  expect(document.querySelector('.fade-button')).toBeNull();
  (document.querySelector('button[aria-label="Text"]') as HTMLButtonElement).click();
  await vi.waitFor(() => expect(document.querySelector('canvas:not(.capture-image)')).not.toBeNull());
  expect(app.state().preferences).toMatchObject({ tool: 'text', toolbarPosition: 'left', autoFadeSeconds: 5 });
  const canvas = document.querySelector('canvas:not(.capture-image)')!;
  canvas.dispatchEvent(Object.assign(new MouseEvent('pointerdown', { bubbles: true, button: 0, clientX: 200, clientY: 200 }), { pointerId: 1 }));
  await tick();
  const input = document.querySelector('textarea')!;
  input.value = 'Explain this'; input.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  input.dispatchEvent(new KeyboardEvent('keydown', { key: 's', metaKey: true, bubbles: true, cancelable: true }));
  await tick(); await tick();
  expect(annotatedCapture).toHaveBeenCalledWith(expect.anything(), { x: 100, y: 100, width: 500, height: 300 }, expect.arrayContaining([expect.objectContaining({ tool: 'text', text: 'Explain this', fadeSeconds: 0 })]));
});

it('cancels selection without writing anything to the clipboard', async () => {
  const app = await setup();
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }));
  await tick();
  expect(app.state().capture).toBeNull();
  expect(copyCaptureImage).not.toHaveBeenCalled();
});

it('replaces the region on every drag while Screenshot remains selected and copies without annotations', async () => {
  const app = await setup();
  expect(document.querySelector('button[aria-label="Screenshot"]')?.getAttribute('aria-pressed')).toBe('true');
  expect(document.querySelector('button[aria-label="Arrow"]')?.getAttribute('aria-pressed')).toBe('false');
  expect(document.querySelector('canvas:not(.capture-image)')).toBeNull();
  select([700, 500], [200, 150]);
  expect(document.querySelector('button[aria-label="Screenshot"]')?.getAttribute('aria-pressed')).toBe('true');
  (document.querySelector('button.copy') as HTMLButtonElement).click();
  await tick(); await tick();
  expect(annotatedCapture).toHaveBeenCalledWith(expect.anything(), { x: 200, y: 150, width: 500, height: 350 }, []);
  expect(app.state()).toMatchObject({ capture: null, mode: 'hidden' });
});

it('uses drawing shortcuts to leave selection and Screenshot to replace annotated areas with a fresh history', async () => {
  await setup();
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key: '1', metaKey: true, bubbles: true, cancelable: true }));
  await tick(); await tick();
  expect(document.querySelector('.selection-surface')).toBeNull();
  expect(document.querySelector('button[aria-label="Arrow"]')?.getAttribute('aria-pressed')).toBe('true');
  const canvas = document.querySelector('canvas:not(.capture-image)')!;
  canvas.setPointerCapture = vi.fn();
  canvas.hasPointerCapture = vi.fn().mockReturnValue(false);
  for (const [type, x, y] of [['pointerdown', 200, 200], ['pointerup', 400, 300]] as const) {
    canvas.dispatchEvent(Object.assign(new MouseEvent(type, { bubbles: true, button: 0, clientX: x, clientY: y }), { pointerId: 1 }));
  }
  await tick();
  expect((document.querySelector('button[aria-label="Undo"]') as HTMLButtonElement).disabled).toBe(false);
  (document.querySelector('button[aria-label="Screenshot"]') as HTMLButtonElement).click();
  await tick();
  expect(document.querySelector('canvas:not(.capture-image)')).toBeNull();
  select([300, 200], [650, 450]);
  (document.querySelector('button[aria-label="Arrow"]') as HTMLButtonElement).click();
  await vi.waitFor(() => expect(document.querySelector('canvas:not(.capture-image)')).not.toBeNull());
  expect((document.querySelector('button[aria-label="Undo"]') as HTMLButtonElement).disabled).toBe(true);
  (document.querySelector('button.copy') as HTMLButtonElement).click();
  await tick(); await tick();
  expect(annotatedCapture).toHaveBeenCalledWith(expect.anything(), { x: 300, y: 200, width: 350, height: 250 }, []);
});


it('waits for pixels before selecting and ignores a frame arriving after cancellation', async () => {
  let finish!: (value: ImageData) => void;
  vi.mocked(getCaptureImage).mockImplementationOnce(() => new Promise(resolve => finish = resolve));
  await setup();
  expect(document.querySelector('.capture-editor')?.classList.contains('ready')).toBe(false);
  expect(document.querySelector('.selection')).toBeNull();
  const image = document.querySelector('.capture-image') as HTMLCanvasElement;
  const context = image.getContext('2d')!;
  await unmount(component!); component = undefined;
  finish({ width: 2560, height: 1600 } as ImageData);
  await tick(); await tick();
  expect(context.putImageData).not.toHaveBeenCalled();
  expect(image.width).toBe(0);
  expect(image.height).toBe(0);
});
