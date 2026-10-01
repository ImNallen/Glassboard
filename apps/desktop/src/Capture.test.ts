// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { fromStore, writable } from 'svelte/store';
import Capture from './Capture.svelte';
import SharedCapture from '@glassboard/ui/Capture.svelte';
import { action, defaults, savePreferences, subscribe, type Session } from '@glassboard/ui/session';
import { annotatedCapture } from '@glassboard/ui/capture';
import { copyCaptureImage, getCaptureImage } from './lib/capture';

vi.mock('./lib/capture', () => ({ getCaptureImage: vi.fn().mockResolvedValue({ width: 2560, height: 1600, data: new Uint8ClampedArray(0) }), copyCaptureImage: vi.fn(), copyCaptureRegion: vi.fn() }));
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

function selectionPointer(type: string, [x, y]: [number, number]) {
  const surface = document.querySelector('.selection-surface') as HTMLElement;
  surface.setPointerCapture = vi.fn();
  surface.dispatchEvent(Object.assign(new MouseEvent(type, { bubbles: true, button: 0, clientX: x, clientY: y }), { pointerId: 1 }));
  flushSync();
}

function select(from: [number, number], to: [number, number]) {
  for (const [type, [x, y]] of [['pointerdown', from], ['pointerup', to]] as const) {
    selectionPointer(type, [x, y]);
  }
}

async function setup(copyRegion?: (id: number, region: { x: number; y: number; width: number; height: number }) => Promise<void>, initialSelection = true) {
  await action('dismiss-tutorial');
  await savePreferences({ ...defaults.preferences, toolbarPosition: 'left', autoFadeSeconds: 5 });
  await action('capture');
  const store = writable<Session>(structuredClone(defaults));
  let session = structuredClone(defaults);
  stop = await subscribe(value => { session = value; store.set(value); });
  const current = fromStore(store);
  const props = { get session() { return current.current; }, onerror: vi.fn() };
  component = copyRegion
    ? mount(SharedCapture, { target: document.body, props: { ...props, get session() { return current.current; }, getImage: getCaptureImage, copyImage: copyCaptureImage, copyRegion } })
    : mount(Capture, { target: document.body, props });
  await tick(); await tick();
  flushSync();
  if (initialSelection) select([100, 100], [600, 400]);
  return { state: () => session };
}

it('uses native crop bounds at source density without composing or encoding plain captures', async () => {
  vi.stubGlobal('innerWidth', 1280); vi.stubGlobal('innerHeight', 800);
  const copyRegion = vi.fn().mockResolvedValue(undefined);
  const app = await setup(copyRegion);
  select([10.25, 20.75], [50.75, 51.25]);
  (document.querySelector('button.copy') as HTMLButtonElement).click();
  await tick(); await tick();
  expect(copyRegion).toHaveBeenCalledWith(expect.any(Number), { x: 20, y: 41, width: 82, height: 62 });
  expect(annotatedCapture).not.toHaveBeenCalled();
  expect(copyCaptureImage).not.toHaveBeenCalled();
  expect(app.state().capture).toBeNull();
});

it('ignores the old clipboard shortcut once Copy is rebound', async () => {
  const copyRegion = vi.fn().mockResolvedValue(undefined);
  const app = await setup(copyRegion);
  await savePreferences({ ...app.state().preferences, keybindings: { copy: 'CommandOrControl+Shift+KeyC' } });
  await tick();
  const clipboard = new Event('copy', { bubbles: true, cancelable: true });
  window.dispatchEvent(clipboard);
  await tick(); await tick();
  expect(clipboard.defaultPrevented).toBe(false);
  expect(copyRegion).not.toHaveBeenCalled();
  expect(app.state().capture).not.toBeNull();
  await savePreferences({ ...app.state().preferences, keybindings: {} });
  await tick();
  window.dispatchEvent(new Event('copy', { bubbles: true, cancelable: true }));
  await tick(); await tick();
  expect(copyRegion).toHaveBeenCalledOnce();
});

it('keeps a failed native crop open for retry', async () => {
  const copyRegion = vi.fn().mockRejectedValueOnce(new Error('Clipboard busy')).mockResolvedValue(undefined);
  const app = await setup(copyRegion);
  (document.querySelector('button.copy') as HTMLButtonElement).click();
  await tick(); await tick();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('Clipboard busy');
  expect(app.state().capture).not.toBeNull();
  (document.querySelector('button.copy') as HTMLButtonElement).click();
  await tick(); await tick();
  expect(copyRegion).toHaveBeenCalledTimes(2);
  expect(app.state().capture).toBeNull();
});

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
  const copyRegion = vi.fn().mockResolvedValue(undefined);
  const app = await setup(copyRegion);
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
  expect(copyCaptureImage).not.toHaveBeenCalled();
  input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
  await tick();
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'c', metaKey: true, bubbles: true, cancelable: true }));
  await tick(); await tick();
  expect(annotatedCapture).toHaveBeenCalledWith(expect.anything(), { x: 100, y: 100, width: 500, height: 300 }, expect.arrayContaining([expect.objectContaining({ tool: 'text', text: 'Explain this', fadeSeconds: 0 })]));
  expect(copyRegion).not.toHaveBeenCalled();
});

it('cancels selection without writing anything to the clipboard', async () => {
  const app = await setup();
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }));
  await tick();
  expect(app.state().capture).toBeNull();
  expect(copyCaptureImage).not.toHaveBeenCalled();
});

it('keeps the toolbar out of initial selection and every drag until a valid area is finished', async () => {
  const app = await setup(undefined, false);
  const tools = document.querySelector('.capture-tools') as HTMLElement;
  expect(tools.hidden).toBe(true);
  select([100, 100], [103, 103]);
  expect(tools.hidden).toBe(true);
  expect(document.querySelector('.selection')).toBeNull();
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }));
  flushSync();
  expect(tools.hidden).toBe(false);
  expect(app.state().capture).not.toBeNull();
  selectionPointer('pointerdown', [10, 120]);
  expect(tools.hidden).toBe(true);
  selectionPointer('pointermove', [600, 400]);
  expect(tools.hidden).toBe(true);
  selectionPointer('pointerup', [600, 400]);
  expect(tools.hidden).toBe(false);
  expect(document.querySelector('button.copy')).not.toBeNull();
});

it('reveals the toolbar on Escape while reselecting and abandons partial drags without copying', async () => {
  const app = await setup();
  const tools = document.querySelector('.capture-tools') as HTMLElement;
  (document.querySelector('button[aria-label="Screenshot"]') as HTMLButtonElement).click();
  flushSync();
  expect(tools.hidden).toBe(true);
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }));
  flushSync();
  expect(tools.hidden).toBe(false);
  expect(document.querySelector('button.copy')).not.toBeNull();
  selectionPointer('pointerdown', [200, 200]);
  selectionPointer('pointermove', [400, 400]);
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }));
  flushSync();
  expect(tools.hidden).toBe(false);
  expect(document.querySelector('.selection')).toBeNull();
  expect(document.querySelector('button.copy')).toBeNull();
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', repeat: true, cancelable: true }));
  selectionPointer('pointerup', [400, 400]);
  expect(app.state().capture).not.toBeNull();
  expect(document.querySelector('.selection')).toBeNull();
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
  const captureShortcut = new KeyboardEvent('keydown', { key: 's', metaKey: true, bubbles: true, cancelable: true });
  document.body.dispatchEvent(captureShortcut);
  expect(captureShortcut.defaultPrevented).toBe(true);
  expect(copyCaptureImage).not.toHaveBeenCalled();
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
