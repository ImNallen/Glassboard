import { beforeEach, expect, it, vi } from 'vitest';
import { OverlayRenderer } from './renderer';
import { render } from './canvas';
import type { Shape } from './shapes';

vi.mock('./canvas', () => ({ render: vi.fn() }));
beforeEach(() => vi.clearAllMocks());

const shape = (id: string, expiresAt?: number): Shape => ({ id, tool: 'pen', width: 4, color: '#000000', points: [{ x: 1, y: 2 }, { x: 10, y: 20 }], expiresAt });
function setup() {
  const cached = {} as CanvasRenderingContext2D;
  const canvas = { width: 0, height: 0, getContext: () => cached } as unknown as HTMLCanvasElement;
  const ctx = { canvas: { width: 200, height: 100 }, setTransform: vi.fn(), clearRect: vi.fn(), drawImage: vi.fn() } as unknown as CanvasRenderingContext2D;
  return { renderer: new OverlayRenderer(() => canvas), ctx, cached, canvas };
}

it('reuses completed drawings while repainting only a moving draft', () => {
  const { renderer, ctx, cached } = setup();
  const shapes = [shape('one'), shape('two')];
  const draft = shape('draft');
  renderer.paint(ctx, shapes, draft, 200, 100, 1, 0);
  renderer.paint(ctx, shapes, draft, 200, 100, 1, 16);
  expect(vi.mocked(render).mock.calls.filter(call => call[0] === cached)).toHaveLength(1);
  expect(vi.mocked(render).mock.calls.filter(call => call[0] === ctx).map(call => [call[1], call[2], call[7]])).toEqual([[[], draft, false], [[], draft, false]]);
  expect(ctx.drawImage).toHaveBeenCalledTimes(2);
});

it('keeps newer translucent drawings above a fading shape', () => {
  const { renderer, ctx, cached } = setup();
  const shapes = [shape('permanent'), shape('fading', 3000), { ...shape('newer'), tool: 'highlighter' as const }];
  renderer.paint(ctx, shapes, null, 200, 100, 1, 2000);
  renderer.paint(ctx, shapes, null, 200, 100, 1, 2750);
  expect(vi.mocked(render).mock.calls.filter(call => call[0] === cached).map(call => call[1].map(s => s.id))).toEqual([['permanent', 'fading', 'newer'], ['permanent']]);
  expect(render).toHaveBeenLastCalledWith(ctx, shapes.slice(1), null, 200, 100, 1, 2750, false);
  renderer.paint(ctx, shapes, null, 200, 100, 1, 2800);
  expect(vi.mocked(render).mock.calls.filter(call => call[0] === cached)).toHaveLength(2);
});

it('invalidates the bitmap for history edits, resizing, and pixel-density changes', () => {
  const { renderer, ctx, canvas, cached } = setup();
  const shapes = [shape('one'), shape('two')];
  renderer.paint(ctx, shapes, null, 200, 100, 1, 0);
  renderer.paint(ctx, shapes, null, 200, 100, 2, 0);
  expect([canvas.width, canvas.height]).toEqual([400, 200]);
  renderer.paint(ctx, shapes, null, 300, 100, 2, 0);
  expect([canvas.width, canvas.height]).toEqual([600, 200]);
  renderer.paint(ctx, shapes.slice(1), null, 300, 100, 2, 0);
  expect(vi.mocked(render).mock.calls.filter(call => call[0] === cached)).toHaveLength(4);
  renderer.paint(ctx, [], null, 300, 100, 2, 0);
  expect([canvas.width, canvas.height]).toEqual([0, 0]);
  expect(render).toHaveBeenLastCalledWith(ctx, [], null, 300, 100, 2, 0);
});

it('releases the bitmap on a session reset and redraws when history is restored', () => {
  const { renderer, ctx, cached, canvas } = setup();
  const shapes = [shape('one')];
  renderer.paint(ctx, shapes, null, 200, 100, 1, 0);
  renderer.reset();
  expect([canvas.width, canvas.height]).toEqual([0, 0]);
  renderer.paint(ctx, shapes, null, 200, 100, 1, 0);
  expect(vi.mocked(render).mock.calls.filter(call => call[0] === cached)).toHaveLength(2);
});

it('uses the uncached renderer when the first shape is fading', () => {
  const { renderer, ctx } = setup();
  const shapes = [shape('fading', 3000), shape('permanent')];
  renderer.paint(ctx, shapes, null, 200, 100, 1, 2750);
  expect(ctx.drawImage).not.toHaveBeenCalled();
  expect(render).toHaveBeenLastCalledWith(ctx, shapes, null, 200, 100, 1, 2750);
});
