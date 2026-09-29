// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { fromStore, writable } from 'svelte/store';
import Overlay from './Overlay.svelte';
import { defaults, reportHistory } from './lib/session';
import { render, shapeAtPoint } from './lib/drawing';

vi.mock('./lib/session', async importOriginal => ({
  ...await importOriginal<typeof import('./lib/session')>(),
  activateOverlay: vi.fn().mockResolvedValue(undefined),
  reportHistory: vi.fn().mockResolvedValue(undefined),
  drawingEvents: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock('./lib/drawing', async importOriginal => ({
  ...await importOriginal<typeof import('./lib/drawing')>(),
  render: vi.fn(),
  shapeAtPoint: vi.fn(),
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

function pointer(canvas: HTMLCanvasElement, type: string, x: number, y: number, init: MouseEventInit = {}) {
  const buttons = type === 'pointerup' || type === 'lostpointercapture' ? 0 : 1;
  canvas.dispatchEvent(Object.assign(new MouseEvent(type, { clientX: x, clientY: y, button: 0, buttons, bubbles: true, ...init }), { pointerId: 1 }));
  flushSync();
  paint();
}

function draw(canvas: HTMLCanvasElement) {
  canvas.setPointerCapture = vi.fn();
  canvas.hasPointerCapture = vi.fn(() => false);
  pointer(canvas, 'pointerdown', 10, 20);
  pointer(canvas, 'pointermove', 110, 80);
  pointer(canvas, 'pointerup', 110, 80);
}

function setup() {
  const store = writable({ ...structuredClone(defaults), mode: 'draw' as const });
  const current = fromStore(store);
  component = mount(Overlay, { target: document.body, props: { get session() { return current.current; }, onerror: vi.fn() } });
  flushSync();
  const canvas = document.querySelector('canvas')!;
  canvas.setPointerCapture = vi.fn();
  canvas.hasPointerCapture = vi.fn(() => false);
  return { store, canvas };
}

/** Committed shapes passed to the last render; the draft travels separately. */
const drawn = () => vi.mocked(render).mock.lastCall?.[1];

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

it('keeps a stroke when pointer capture is lost before the pointer is released', () => {
  const { canvas } = setup();
  pointer(canvas, 'pointerdown', 10, 20);
  pointer(canvas, 'pointermove', 110, 80);
  expect(drawn()).toHaveLength(0);
  pointer(canvas, 'lostpointercapture', 110, 80);
  expect(drawn()).toHaveLength(1);
  expect(reportHistory).toHaveBeenLastCalledWith({ canUndo: true, canRedo: false }, 0, false);
});

it('keeps a stroke when the button was released where the page could not see it', () => {
  const { canvas } = setup();
  pointer(canvas, 'pointerdown', 10, 20);
  pointer(canvas, 'pointermove', 110, 80);
  // The cursor comes back with no button held: the stroke ends where it was last seen.
  pointer(canvas, 'pointermove', 400, 400, { buttons: 0 });
  const shapes = drawn()!;
  expect(shapes).toHaveLength(1);
  expect(shapes[0].points[shapes[0].points.length - 1]).toEqual({ x: 110, y: 80 });
  // A fresh press starts a new stroke rather than being swallowed by the old one.
  pointer(canvas, 'pointerdown', 200, 200);
  pointer(canvas, 'pointermove', 300, 300);
  pointer(canvas, 'pointerup', 300, 300);
  expect(drawn()).toHaveLength(2);
});

it('commits an erase when capture is lost mid-drag instead of restoring the shapes', () => {
  const { store, canvas } = setup();
  draw(canvas);
  const [shape] = drawn()!;
  store.update(session => ({ ...session, preferences: { ...session.preferences, tool: 'eraser' } }));
  flushSync();
  vi.mocked(shapeAtPoint).mockReturnValue(shape);
  pointer(canvas, 'pointerdown', 50, 50);
  expect(drawn()).toHaveLength(0);
  pointer(canvas, 'lostpointercapture', 50, 50);
  expect(drawn()).toHaveLength(0);
  expect(reportHistory).toHaveBeenLastCalledWith({ canUndo: true, canRedo: false }, 0, false);
});

it('discards the stroke on pointercancel, when the browser took the gesture', () => {
  const { canvas } = setup();
  pointer(canvas, 'pointerdown', 10, 20);
  pointer(canvas, 'pointermove', 110, 80);
  pointer(canvas, 'pointercancel', 110, 80);
  expect(drawn()).toHaveLength(0);
});
