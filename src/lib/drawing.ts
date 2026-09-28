export type Tool = 'arrow' | 'rectangle' | 'ellipse' | 'highlighter';
export type ColorMode = 'solid' | 'rainbow' | 'cycle';
export type AutoFadeSeconds = 0 | 3 | 5 | 10;
export const AUTO_FADE_OPTIONS: readonly AutoFadeSeconds[] = [0, 3, 5, 10];
const FADE_MS = 500;
export type Point = { x: number; y: number };
export type Shape = { id: string; tool: Tool; color: string; width: number; points: Point[]; colorMode?: ColorMode; hue?: number; fadeSeconds?: AutoFadeSeconds; expiresAt?: number };
export function shapeOpacity(shape: Shape, now: number): number {
  return shape.expiresAt === undefined ? 1 : Math.max(0, Math.min(1, (shape.expiresAt - now) / FADE_MS));
}
export const PALETTE = [
  ['#f46b78', 'Coral red'], ['#f2c85b', 'Warm yellow'], ['#4dcaa0', 'Mint green'],
  ['#4fc5d5', 'Aqua'], ['#669df0', 'Sky blue'], ['#a184e8', 'Violet'],
  ['#e580b5', 'Pink'],
] as const;
export const CYCLE_COLORS = PALETTE.map(([color]) => color);
export const RAINBOW_PREVIEW = `conic-gradient(${[...CYCLE_COLORS, CYCLE_COLORS[0]].join(', ')})`;
export function cycleColor(index: number): string { return CYCLE_COLORS[index % CYCLE_COLORS.length]; }

export function createShape(style: { tool: Tool; color: string; width: number; colorMode: ColorMode; autoFadeSeconds?: AutoFadeSeconds }, cycleIndex: number, start: Point): Shape {
  return {
    id: crypto.randomUUID(), tool: style.tool, width: style.width,
    color: style.colorMode === 'cycle' ? cycleColor(cycleIndex) : style.color,
    colorMode: style.colorMode, hue: style.colorMode === 'rainbow' ? Math.random() * 360 : 0, points: [{ ...start }],
    fadeSeconds: style.autoFadeSeconds ?? 0,
  };
}

export function shiftRainbow(shape: Shape, from: Point, to: Point) {
  if (shape.colorMode === 'rainbow') {
    shape.hue = ((shape.hue ?? 0) + Math.hypot(to.x - from.x, to.y - from.y) * .6) % 360;
  }
}

/** Use consistent bounds for closed shapes while preserving an arrow's direction. */
export function rainbowAxis(shape: Shape): [Point, Point] {
  const start = shape.points[0] ?? { x: 0, y: 0 };
  let from = { ...start }, to = { ...(shape.points.at(-1) ?? start) };
  if (shape.tool !== 'arrow') {
    for (const point of shape.points) {
      from.x = Math.min(from.x, point.x); from.y = Math.min(from.y, point.y);
      to.x = Math.max(to.x, point.x); to.y = Math.max(to.y, point.y);
    }
  }
  if (Math.hypot(to.x - from.x, to.y - from.y) < 1) to.x = from.x + Math.max(shape.width, 1);
  return [from, to];
}

function strokeColor(ctx: CanvasRenderingContext2D, shape: Shape): string | CanvasGradient {
  if (shape.colorMode !== 'rainbow') return shape.color;
  const [from, to] = rainbowAxis(shape);
  const gradient = ctx.createLinearGradient(from.x, from.y, to.x, to.y);
  // Rotate the chosen palette, interpolating only the wraparound endpoints.
  const offset = ((shape.hue ?? 0) / 360 % 1) * CYCLE_COLORS.length;
  const index = Math.floor(offset), fraction = offset - index;
  const fromColor = parseInt(cycleColor(index).slice(1), 16);
  const toColor = parseInt(cycleColor(index + 1).slice(1), 16);
  const channels = [16, 8, 0].map(shift => Math.round(
    ((fromColor >> shift) & 255) * (1 - fraction) + ((toColor >> shift) & 255) * fraction,
  ));
  const edge = `rgb(${channels.join(', ')})`;
  gradient.addColorStop(0, edge);
  for (let i = 1; i <= CYCLE_COLORS.length; i++) {
    gradient.addColorStop((i - fraction) / CYCLE_COLORS.length, cycleColor(index + i));
  }
  gradient.addColorStop(1, edge);
  return gradient;
}

export function constrainEnd(start: Point, end: Point, tool: Tool, shift: boolean): Point {
  if (!shift || tool === 'highlighter') return end;
  const dx = end.x - start.x, dy = end.y - start.y;
  if (tool === 'arrow') {
    const length = Math.hypot(dx, dy);
    const angle = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
    return { x: start.x + Math.cos(angle) * length, y: start.y + Math.sin(angle) * length };
  }
  const size = Math.max(Math.abs(dx), Math.abs(dy));
  return { x: start.x + Math.sign(dx || 1) * size, y: start.y + Math.sign(dy || 1) * size };
}

export class DrawingHistory {
  shapes: Shape[] = [];
  private past: Shape[][] = [];
  private future: Shape[][] = [];
  get canUndo() { return this.past.length > 0; }
  get canRedo() { return this.future.length > 0; }
  private commit(next: Shape[]) {
    this.past.push(this.shapes);
    if (this.past.length > 100) this.past.shift();
    this.shapes = next;
    this.future = [];
  }
  add(shape: Shape, now = Date.now()) {
    this.commit([...this.shapes, { ...shape, points: shape.points.map(p => ({ ...p })),
      expiresAt: shape.fadeSeconds ? now + shape.fadeSeconds * 1000 : undefined }]);
  }
  expire(now: number): boolean {
    const alive = (shapes: Shape[]) => shapes.filter(shape => shape.expiresAt === undefined || shape.expiresAt > now);
    const same = (a: Shape[], b: Shape[]) => a.length === b.length && a.every((shape, i) => shape.id === b[i].id);
    const before = [this.shapes.length, this.canUndo, this.canRedo];
    this.shapes = alive(this.shapes);
    // Remove expired shapes from every snapshot so undo/redo cannot resurrect them.
    // Collapse empty history steps while preserving edits to permanent drawings.
    const prune = (stack: Shape[][]) => {
      let previous = this.shapes;
      const result: Shape[][] = [];
      for (let i = stack.length - 1; i >= 0; i--) {
        const next = alive(stack[i]);
        if (!same(next, previous)) { result.push(next); previous = next; }
      }
      return result.reverse();
    };
    this.past = prune(this.past); this.future = prune(this.future);
    return before[0] !== this.shapes.length || before[1] !== this.canUndo || before[2] !== this.canRedo;
  }
  nextFadeUpdate(now: number): number | undefined {
    let next = Infinity;
    for (const shape of this.shapes) {
      if (shape.expiresAt !== undefined) next = Math.min(next, Math.max(now, shape.expiresAt - FADE_MS));
    }
    for (const stack of [this.past, this.future]) for (const shapes of stack) for (const shape of shapes) {
      if (shape.expiresAt !== undefined) next = Math.min(next, shape.expiresAt);
    }
    return Number.isFinite(next) ? next : undefined;
  }
  clear() { if (this.shapes.length) this.commit([]); }
  remove(id: string): boolean {
    const next = this.shapes.filter(shape => shape.id !== id);
    if (next.length === this.shapes.length) return false;
    this.commit(next);
    return true;
  }
  undo() { const previous = this.past.pop(); if (previous) { this.future.push(this.shapes); this.shapes = previous; } }
  redo() { const next = this.future.pop(); if (next) { this.past.push(this.shapes); this.shapes = next; } }
}

function path(ctx: CanvasRenderingContext2D, shape: Shape) {
  const points = shape.points, start = points[0], end = points[points.length - 1];
  if (!start || !end) return;
  ctx.beginPath();
  if (shape.tool === 'rectangle') {
    const width = Math.abs(end.x - start.x), height = Math.abs(end.y - start.y);
    const radius = Math.min(shape.width * 2, width / 2, height / 2);
    ctx.roundRect(Math.min(start.x, end.x), Math.min(start.y, end.y), width, height, radius);
  } else if (shape.tool === 'ellipse') {
    ctx.ellipse((start.x + end.x) / 2, (start.y + end.y) / 2, Math.abs(end.x - start.x) / 2, Math.abs(end.y - start.y) / 2, 0, 0, Math.PI * 2);
  } else if (shape.tool === 'arrow') {
    const angle = Math.atan2(end.y - start.y, end.x - start.x);
    const length = Math.hypot(end.x - start.x, end.y - start.y);
    if (length < 1) { ctx.arc(start.x, start.y, shape.width / 2, 0, Math.PI * 2); return; }
    const head = Math.min(length * .45, 12 + shape.width * 2.2);
    const radius = Math.min(shape.width / 2, length / 4);
    const wing = Math.max(head * .5, radius * 1.6), neck = length - head;
    // One filled silhouette keeps the rounded head, shaft, glow, and shadow seamless.
    const vertices = [[0, -radius], [neck, -radius], [neck, -wing], [length, 0],
      [neck, wing], [neck, radius], [0, radius]].map(([x, y]) => ({
      x: start.x + x * Math.cos(angle) - y * Math.sin(angle),
      y: start.y + x * Math.sin(angle) + y * Math.cos(angle),
    }));
    vertices.forEach((vertex, i) => {
      const previous = vertices[(i + vertices.length - 1) % vertices.length];
      const next = vertices[(i + 1) % vertices.length];
      const before = Math.hypot(previous.x - vertex.x, previous.y - vertex.y);
      const after = Math.hypot(next.x - vertex.x, next.y - vertex.y);
      const rounding = Math.min(radius, before / 2, after / 2);
      const entry = { x: vertex.x + (previous.x - vertex.x) * rounding / before,
        y: vertex.y + (previous.y - vertex.y) * rounding / before };
      if (i === 0) ctx.moveTo(entry.x, entry.y); else ctx.lineTo(entry.x, entry.y);
      ctx.quadraticCurveTo(vertex.x, vertex.y,
        vertex.x + (next.x - vertex.x) * rounding / after,
        vertex.y + (next.y - vertex.y) * rounding / after);
    });
    ctx.closePath();
  } else {
    ctx.moveTo(start.x, start.y);
    if (points.length === 1) ctx.lineTo(start.x + .01, start.y);
    for (let i = 1; i < points.length - 1; i++) {
      const next = points[i + 1];
      ctx.quadraticCurveTo(points[i].x, points[i].y, (points[i].x + next.x) / 2, (points[i].y + next.y) / 2);
    }
    if (points.length > 1) ctx.lineTo(end.x, end.y);
  }
}
/** Hit the visible geometry, newest first, with a small allowance around thin strokes. */
export function shapeAtPoint(ctx: CanvasRenderingContext2D, shapes: Shape[], point: Point, now = Date.now()): Shape | undefined {
  ctx.save();
  try {
    // Pointer positions and shape geometry are both in CSS pixels, regardless of display scale.
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    for (let i = shapes.length - 1; i >= 0; i--) {
      const shape = shapes[i];
      if (!shape.points.length || shapeOpacity(shape, now) === 0) continue;
      path(ctx, shape);
      ctx.lineWidth = (shape.tool === 'arrow' ? 0 : shape.width * (shape.tool === 'highlighter' ? 5 : 1)) + 12;
      if ((shape.tool === 'arrow' && ctx.isPointInPath(point.x, point.y)) || ctx.isPointInStroke(point.x, point.y)) return shape;
    }
  } finally { ctx.restore(); }
}

export function render(ctx: CanvasRenderingContext2D, shapes: Shape[], draft: Shape | null, width: number, height: number, scale: number, now = Date.now()) {
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.clearRect(0, 0, width, height);
  for (const shape of draft ? [...shapes, draft] : shapes) {
    const opacity = shapeOpacity(shape, now);
    if (opacity === 0) continue;
    ctx.save();
    ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    path(ctx, shape);
    const color = strokeColor(ctx, shape);
    if (shape.tool === 'highlighter') {
      ctx.globalAlpha = .35 * opacity; ctx.lineWidth = shape.width * 5; ctx.strokeStyle = color; ctx.stroke();
    } else {
      // Feather the same stroke color up to 3 px outward, including every rainbow segment.
      ctx.strokeStyle = color;
      ctx.globalAlpha = .055 * opacity;
      for (let spread = 6; spread >= 1; spread--) {
        ctx.lineWidth = (shape.tool === 'arrow' ? 0 : shape.width) + spread;
        ctx.stroke();
      }
      ctx.globalAlpha = opacity;
      // Canvas shadows use device pixels independently of the drawing transform.
      ctx.shadowColor = 'rgba(25, 30, 40, .18)';
      ctx.shadowBlur = 3 * scale;
      ctx.shadowOffsetY = scale;
      if (shape.tool === 'arrow') {
        ctx.fillStyle = color; ctx.fill();
      } else {
        ctx.lineWidth = shape.width; ctx.strokeStyle = color; ctx.stroke();
      }
    }
    ctx.restore();
  }
}
