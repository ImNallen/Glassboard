import { describe, expect, it, vi } from 'vitest';
import { DrawingHistory, constrainEnd, createShape, cycleColor, CYCLE_COLORS, rainbowAxis, shiftRainbow, render, shapeOpacity, textFontSize, textLineHeight, type Shape, type Tool } from './drawing';
const arrow = (): Shape => ({ id: 'one', tool: 'arrow', color: '#ff6259', width: 4, points: [{ x: 0, y: 0 }, { x: 40, y: 20 }] });
describe('auto-fade', () => {
  it('retains the renderable array until a deadline is due, including during fading', () => {
    const history = new DrawingHistory();
    history.add({ ...arrow(), fadeSeconds: 3 }, 0);
    const shapes = history.shapes;
    for (const now of [1, 2000, 2500, 2750, 2999]) {
      expect(history.expire(now)).toBe(false);
      expect(history.shapes).toBe(shapes);
    }
    expect(history.expire(3000)).toBe(true);
    expect(history.shapes).not.toBe(shapes);
  });
  it.each([3, 5, 10] as const)('starts the %ss timer on completion and fades only at the end', seconds => {
    const draft = createShape({ ...arrow(), colorMode: 'rainbow', autoFadeSeconds: seconds }, 0, { x: 0, y: 0 });
    expect(shapeOpacity(draft, 999999)).toBe(1);
    const history = new DrawingHistory();
    history.add(draft, 1000);
    const shape = history.shapes[0], end = 1000 + seconds * 1000;
    expect(shape.expiresAt).toBe(end);
    expect(shapeOpacity(shape, end - 501)).toBe(1);
    expect(shapeOpacity(shape, end - 250)).toBe(.5);
    expect(shapeOpacity(shape, end)).toBe(0);
    expect(history.nextFadeUpdate(1000)).toBe(end - 500);
    expect(history.nextFadeUpdate(end - 250)).toBe(end - 250);
    expect(history.expire(end)).toBe(true);
    expect(history.shapes).toEqual([]);
    expect([history.canUndo, history.canRedo]).toEqual([false, false]);
    expect(history.nextFadeUpdate(end)).toBeUndefined();
  });
  it('keeps infinite drawings and does not schedule animation for them', () => {
    const history = new DrawingHistory(); history.add(arrow(), 0);
    expect(history.expire(999999)).toBe(false);
    expect(shapeOpacity(history.shapes[0], 999999)).toBe(1);
    expect(history.nextFadeUpdate(999999)).toBeUndefined();
  });
  it('preserves permanent history and never restores expired drawings through clear/undo/redo', () => {
    const history = new DrawingHistory();
    history.add({ ...arrow(), id: 'permanent' }, 0);
    history.add({ ...arrow(), id: 'temporary', fadeSeconds: 3 }, 0);
    history.clear();
    expect(history.nextFadeUpdate(1000)).toBe(3000);
    history.expire(3000);
    history.undo(); expect(history.shapes.map(shape => shape.id)).toEqual(['permanent']);
    history.undo(); expect(history.shapes).toEqual([]);
    expect(history.canUndo).toBe(false);
    history.redo(); expect(history.shapes.map(shape => shape.id)).toEqual(['permanent']);
    history.redo(); expect(history.shapes).toEqual([]);
    expect(history.canRedo).toBe(false);
  });
  it('keeps original deadlines through undo/redo and prunes expired redo entries', () => {
    const history = new DrawingHistory(); history.add({ ...arrow(), fadeSeconds: 3 }, 0);
    history.undo(); history.expire(1000); history.redo();
    expect(history.shapes[0].expiresAt).toBe(3000);
    history.undo(); history.expire(3000); history.redo();
    expect(history.shapes).toEqual([]); expect(history.canRedo).toBe(false);
  });
  it('fades the main stroke, glow, and highlighter opacity together', () => {
    for (const tool of ['rectangle', 'highlighter'] as const) {
      const alphas: number[] = [];
      const ctx = {
        setTransform: vi.fn(), clearRect: vi.fn(), save: vi.fn(), restore: vi.fn(), beginPath: vi.fn(),
        roundRect: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), globalAlpha: 1,
        stroke() { alphas.push(this.globalAlpha); },
      };
      render(ctx as unknown as CanvasRenderingContext2D,
        [{ ...arrow(), tool, expiresAt: 3000 }], null, 100, 100, 1, 2750);
      expect(alphas.at(-1)).toBe(tool === 'highlighter' ? .175 : .5);
      if (tool === 'rectangle') expect(alphas.slice(0, -1)).toEqual(Array(6).fill(.0275));
    }
  });
});
describe('annotation history', () => {
  it('starts a fresh session with no drawings, fading timers, or recoverable history', () => {
    const h = new DrawingHistory();
    h.add({ ...arrow(), fadeSeconds: 3 }, 0);
    h.add({ ...arrow(), id: 'two' }, 0);
    h.undo();
    expect([h.canUndo, h.canRedo]).toEqual([true, true]);
    h.reset();
    h.undo(); h.redo();
    expect(h.shapes).toEqual([]);
    expect([h.canUndo, h.canRedo]).toEqual([false, false]);
    expect(h.nextFadeUpdate(0)).toBeUndefined();
    h.add({ ...arrow(), id: 'new' });
    h.undo(); h.redo();
    expect(h.shapes.map(shape => shape.id)).toEqual(['new']);
  });
  it('restores drawings after clear, undo, and redo', () => {
    const h = new DrawingHistory();
    expect([h.canUndo, h.canRedo]).toEqual([false, false]);
    h.add(arrow());
    expect([h.canUndo, h.canRedo]).toEqual([true, false]);
    h.clear();
    expect(h.shapes).toHaveLength(0);
    expect([h.canUndo, h.canRedo]).toEqual([true, false]);
    h.undo(); expect(h.shapes).toHaveLength(1);
    expect([h.canUndo, h.canRedo]).toEqual([true, true]);
    h.undo(); expect(h.shapes).toHaveLength(0);
    expect([h.canUndo, h.canRedo]).toEqual([false, true]);
    h.redo(); expect(h.shapes).toHaveLength(1);
    expect([h.canUndo, h.canRedo]).toEqual([true, true]);
    h.redo(); expect(h.shapes).toHaveLength(0);
    expect([h.canUndo, h.canRedo]).toEqual([true, false]);
  });
  it('discards the redo branch after a new stroke and isolates committed points', () => {
    const h = new DrawingHistory(), shape = arrow(); h.add(shape);
    shape.points[0].x = 99; expect(h.shapes[0].points[0].x).toBe(0);
    h.undo(); h.add({ ...arrow(), id: 'two' }); h.redo();
    expect(h.shapes.map(s => s.id)).toEqual(['two']); expect(h.canRedo).toBe(false);
  });
  it('keeps an empty clear from swallowing the last undo', () => {
    const h = new DrawingHistory(); h.add(arrow()); h.clear(); h.clear(); h.undo(); expect(h.shapes).toHaveLength(1);
  });
  it('erases one shape and restores its stacking order through undo/redo', () => {
    const h = new DrawingHistory();
    for (const id of ['first', 'middle', 'last']) h.add({ ...arrow(), id });
    expect(h.remove('middle')).toBe(true);
    expect(h.shapes.map(shape => shape.id)).toEqual(['first', 'last']);
    h.undo(); expect(h.shapes.map(shape => shape.id)).toEqual(['first', 'middle', 'last']);
    h.redo(); expect(h.shapes.map(shape => shape.id)).toEqual(['first', 'last']);
  });
  it('does not create history for a missed erase or discard redo', () => {
    const h = new DrawingHistory(); h.add(arrow()); h.undo();
    expect(h.remove('missing')).toBe(false);
    expect(h.canUndo).toBe(false); expect(h.canRedo).toBe(true);
    h.redo(); expect(h.shapes).toHaveLength(1);
  });
  it('preserves a deleted drawing’s fade deadline and cannot restore it after expiry', () => {
    const h = new DrawingHistory(); h.add({ ...arrow(), fadeSeconds: 3 }, 0);
    h.remove('one'); h.undo(); expect(h.shapes[0].expiresAt).toBe(3000);
    h.redo(); h.expire(3000); h.undo();
    expect(h.shapes).toEqual([]); expect(h.canUndo).toBe(false);
  });
});

describe('rainbow and cycling colors', () => {
  const style = { tool: 'arrow' as const, color: '#ffffff' };

  it('cycles through the palette without changing existing shapes', () => {
    const shapes = Array.from({ length: CYCLE_COLORS.length + 1 }, (_, i) => createShape({ ...style, colorMode: 'cycle' }, i, { x: 0, y: 0 }));
    expect(shapes.slice(0, -1).map(shape => shape.color)).toEqual([...CYCLE_COLORS]);
    expect(shapes.at(-1)!.color).toBe(shapes[0].color);
    expect(cycleColor(2 * CYCLE_COLORS.length)).toBe(CYCLE_COLORS[0]);
    const history = new DrawingHistory();
    history.add(shapes[1]); history.undo(); history.redo();
    expect(history.shapes[0].color).toBe(CYCLE_COLORS[1]);
  });

  it.each(['arrow', 'rectangle', 'ellipse', 'highlighter'] as Tool[])('freezes the dragged rainbow on a completed %s, including undo/redo', tool => {
    const shape = createShape({ ...style, tool, colorMode: 'rainbow' }, 0, { x: 0, y: 0 });
    const startingHue = shape.hue;
    shiftRainbow(shape, { x: 0, y: 0 }, { x: 100, y: 0 });
    expect(shape.hue).not.toBe(startingHue);
    expect(shape.hue).toBeGreaterThanOrEqual(0);
    expect(shape.hue).toBeLessThan(360);
    const hue = shape.hue;
    const history = new DrawingHistory(); history.add(shape);
    shiftRainbow(shape, { x: 100, y: 0 }, { x: 200, y: 0 });
    expect(shape.hue).not.toBe(hue);
    history.undo(); history.redo();
    expect(history.shapes[0].hue).toBe(hue);
    expect(history.shapes[0].colorMode).toBe('rainbow');
  });

  it('keeps solid colors unchanged while dragging', () => {
    const shape = createShape({ ...style, colorMode: 'solid' }, 3, { x: 0, y: 0 });
    shiftRainbow(shape, { x: 0, y: 0 }, { x: 100, y: 100 });
    expect(shape.color).toBe('#ffffff'); expect(shape.hue).toBe(0);
  });

  it('uses the drawing direction for a vertical or backwards arrow', () => {
    const shape = { ...arrow(), points: [{ x: 50, y: 200 }, { x: 50, y: 0 }] };
    expect(rainbowAxis(shape)).toEqual(shape.points);
    shape.points = [{ x: 200, y: 30 }, { x: 0, y: 30 }];
    expect(rainbowAxis(shape)).toEqual(shape.points);
  });

  it.each([2, 4, 7])('renders identical boxes in every drag direction at width %s', width => {
    const corners = [
      [{ x: 20, y: 30 }, { x: 220, y: 130 }],
      [{ x: 20, y: 130 }, { x: 220, y: 30 }],
      [{ x: 220, y: 30 }, { x: 20, y: 130 }],
      [{ x: 220, y: 130 }, { x: 20, y: 30 }],
    ];
    const results = corners.map(points => {
      const stops = vi.fn();
      const strokes: number[][] = [];
      const ctx = {
        setTransform: vi.fn(), clearRect: vi.fn(), save: vi.fn(), restore: vi.fn(),
        beginPath: vi.fn(), roundRect: vi.fn(), globalAlpha: 1, lineWidth: 1,
        createLinearGradient: vi.fn(() => ({ addColorStop: stops })),
        stroke() { strokes.push([this.globalAlpha, this.lineWidth]); },
      };
      render(ctx as unknown as CanvasRenderingContext2D,
        [{ ...arrow(), tool: 'rectangle', colorMode: 'rainbow', hue: 80, width, points }], null, 300, 200, 2);
      expect(strokes.at(-1)).toEqual([1, width]);
      return { path: ctx.roundRect.mock.calls, gradient: ctx.createLinearGradient.mock.calls,
        stops: stops.mock.calls, strokes };
    });
    for (const result of results.slice(1)) expect(result).toEqual(results[0]);
  });

  it('gives closed freehand paths and single dots a nonzero gradient', () => {
    const shape: Shape = { ...arrow(), tool: 'highlighter', points: [{ x: 0, y: 0 }, { x: 100, y: 100 }, { x: 0, y: 0 }] };
    expect(rainbowAxis(shape)).toEqual([{ x: 0, y: 0 }, { x: 100, y: 100 }]);
    shape.points = [{ x: 10, y: 20 }];
    const [from, to] = rainbowAxis(shape);
    expect(Math.hypot(to.x - from.x, to.y - from.y)).toBeGreaterThan(0);
  });
});
describe('shift constraints', () => {
  it('keeps a square anchored when dragged up and left', () => {
    expect(constrainEnd({ x: 50, y: 50 }, { x: 10, y: 30 }, 'rectangle', true)).toEqual({ x: 10, y: 10 });
  });
  it('snaps arrows to 45 degrees while retaining length', () => {
    const p = constrainEnd({ x: 0, y: 0 }, { x: 100, y: 80 }, 'arrow', true);
    expect(p.x).toBeCloseTo(p.y); expect(Math.hypot(p.x, p.y)).toBeCloseTo(Math.hypot(100, 80));
  });
});

describe('eraser tool', () => {
  it('removes every swept shape as a single undo step and ignores unknown ids', () => {
    const history = new DrawingHistory();
    for (const id of ['a', 'b', 'c']) history.add({ ...arrow(), id }, 0);
    expect(history.removeAll(['a', 'c', 'missing'])).toBe(true);
    expect(history.shapes.map(shape => shape.id)).toEqual(['b']);
    history.undo();
    expect(history.shapes.map(shape => shape.id)).toEqual(['a', 'b', 'c']);
    expect(history.removeAll(['missing'])).toBe(false);
    expect(history.canRedo).toBe(true);
  });
});

describe('pen and text tools', () => {
  it('lets the pen draw freehand without shift constraints', () => {
    expect(constrainEnd({ x: 0, y: 0 }, { x: 30, y: 7 }, 'pen', true)).toEqual({ x: 30, y: 7 });
    expect(constrainEnd({ x: 0, y: 0 }, { x: 30, y: 7 }, 'rectangle', true)).toEqual({ x: 30, y: 30 });
  });
  it('uses Regular text size and keeps the text on the committed shape', () => {
    const history = new DrawingHistory();
    const shape = createShape({ tool: 'text', color: '#000000', colorMode: 'solid' }, 0, { x: 10, y: 20 });
    expect(shape.width).toBe(4);
    expect(textFontSize(shape.width)).toBe(24);
    expect(textLineHeight(shape.width)).toBe(30);
    shape.text = 'Hello'; shape.points.push({ x: 90, y: 50 });
    history.add(shape, 0);
    expect(history.shapes[0].text).toBe('Hello');
    expect(history.shapes[0].points).toEqual([{ x: 10, y: 20 }, { x: 90, y: 50 }]);
    expect(rainbowAxis(history.shapes[0])).toEqual([{ x: 10, y: 20 }, { x: 90, y: 50 }]);
  });
  it('renders text with fill and glow instead of a stroked path', () => {
    const calls: string[] = [];
    const ctx = {
      setTransform: vi.fn(), clearRect: vi.fn(), save: vi.fn(), restore: vi.fn(), beginPath: vi.fn(), rect: vi.fn(),
      strokeText: () => calls.push('strokeText'), fillText: () => calls.push('fillText'), stroke: () => calls.push('stroke'), fill: () => calls.push('fill'),
    };
    render(ctx as unknown as CanvasRenderingContext2D,
      [{ ...arrow(), tool: 'text', text: 'Hi\nthere', points: [{ x: 0, y: 0 }, { x: 60, y: 60 }] }], null, 100, 100, 1, 0);
    expect(calls.filter(call => call === 'strokeText')).toHaveLength(12);
    expect(calls.filter(call => call === 'fillText')).toHaveLength(2);
    expect(calls).not.toContain('stroke');
    expect(calls).not.toContain('fill');
  });
});
