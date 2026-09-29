// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { fromStore, writable } from 'svelte/store';
import Overlay from './Overlay.svelte';
import { defaults, reportHistory } from './lib/session';
import { render } from './lib/drawing';

vi.mock('./lib/session', async importOriginal => ({
  ...await importOriginal<typeof import('./lib/session')>(),
  activateOverlay: vi.fn().mockResolvedValue(undefined),
  reportHistory: vi.fn().mockResolvedValue(undefined),
  drawingEvents: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock('./lib/drawing', async importOriginal => ({
  ...await importOriginal<typeof import('./lib/drawing')>(),
  render: vi.fn(),
}));

let component: ReturnType<typeof mount> | undefined;
let frames: Map<number, FrameRequestCallback>;
let clearRect: ReturnType<typeof vi.fn>;

beforeEach(() => {
  vi.clearAllMocks();
  frames = new Map();
  let nextFrame = 0;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    frames.set(++nextFrame, callback);
    return nextFrame;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
  clearRect = vi.fn();
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({ clearRect } as unknown as CanvasRenderingContext2D);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function paint() {
  const pending = [...frames.values()];
  frames.clear();
  pending.forEach(callback => callback(0));
}

function draw(canvas: HTMLCanvasElement) {
  canvas.setPointerCapture = vi.fn();
  canvas.hasPointerCapture = vi.fn(() => false);
  for (const [type, x, y] of [['pointerdown', 10, 20], ['pointermove', 110, 80], ['pointerup', 110, 80]] as const) {
    canvas.dispatchEvent(Object.assign(new MouseEvent(type, { clientX: x, clientY: y, button: 0, bubbles: true }), { pointerId: 1 }));
  }
  flushSync();
  paint();
}

it('mounts, draws, clears while frames are paused, and draws again after reopening', () => {
  const store = writable(structuredClone(defaults));
  const current = fromStore(store);
  const onerror = vi.fn();
  component = mount(Overlay, { target: document.body, props: { get session() { return current.current; }, onerror } });
  flushSync();
  const canvas = document.querySelector('canvas')!;
  expect(canvas).not.toBeNull();
  store.update(session => ({ ...session, mode: 'draw' }));
  flushSync();
  draw(canvas);
  expect(vi.mocked(render).mock.lastCall?.[1]).toHaveLength(1);
  expect(reportHistory).toHaveBeenLastCalledWith({ canUndo: true, canRedo: false }, 0, false);

  clearRect.mockClear();
  store.update(session => ({ ...session, mode: 'hidden', annotationSession: 1 }));
  flushSync();
  // No animation frame is run while hidden: the bitmap must still clear.
  expect(clearRect).toHaveBeenCalled();
  expect(reportHistory).toHaveBeenLastCalledWith({ canUndo: false, canRedo: false }, 1, false);

  store.update(session => ({ ...session, mode: 'draw' }));
  flushSync();
  draw(canvas);
  expect(vi.mocked(render).mock.lastCall?.[1]).toHaveLength(1);
  expect(reportHistory).toHaveBeenLastCalledWith({ canUndo: true, canRedo: false }, 1, false);
  expect(onerror).not.toHaveBeenCalled();
});
