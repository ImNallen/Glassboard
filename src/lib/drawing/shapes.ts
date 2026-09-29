export type Tool = 'pen' | 'arrow' | 'rectangle' | 'ellipse' | 'highlighter' | 'text' | 'eraser';
/** Tools that create shapes; the eraser only removes them. */
export const DRAWING_TOOLS: readonly Tool[] = ['pen', 'arrow', 'rectangle', 'ellipse', 'highlighter', 'text'];
/** Freehand tools append points as the pointer moves instead of anchoring a start and end. */
export const FREEHAND_TOOLS: readonly Tool[] = ['pen', 'highlighter'];
export type ColorMode = 'solid' | 'rainbow' | 'cycle';
export type AutoFadeSeconds = 0 | 3 | 5 | 10;
export const AUTO_FADE_OPTIONS: readonly AutoFadeSeconds[] = [0, 3, 5, 10];
export const FADE_MS = 500;
export const REGULAR_WIDTH = 4;
export type Point = { x: number; y: number };
export type Shape = {
  id: string;
  tool: Tool;
  color: string;
  width: number;
  points: Point[];
  colorMode?: ColorMode;
  hue?: number;
  fadeSeconds?: AutoFadeSeconds;
  expiresAt?: number;
  text?: string;
};

type DrawingStyle = {
  tool: Tool;
  color: string;
  colorMode: ColorMode;
  autoFadeSeconds?: AutoFadeSeconds;
};

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

export function createShape(style: DrawingStyle, cycleIndex: number, start: Point): Shape {
  return {
    id: crypto.randomUUID(),
    tool: style.tool,
    width: REGULAR_WIDTH,
    color: style.colorMode === 'cycle' ? cycleColor(cycleIndex) : style.color,
    colorMode: style.colorMode,
    hue: style.colorMode === 'rainbow' ? Math.random() * 360 : 0,
    points: [{ ...start }],
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
  const from = { ...start };
  const to = { ...(shape.points.at(-1) ?? start) };
  if (shape.tool !== 'arrow') {
    for (const point of shape.points) {
      from.x = Math.min(from.x, point.x);
      from.y = Math.min(from.y, point.y);
      to.x = Math.max(to.x, point.x);
      to.y = Math.max(to.y, point.y);
    }
  }
  if (Math.hypot(to.x - from.x, to.y - from.y) < 1) to.x = from.x + Math.max(shape.width, 1);
  return [from, to];
}

export function constrainEnd(start: Point, end: Point, tool: Tool, shift: boolean): Point {
  if (!shift || FREEHAND_TOOLS.includes(tool)) return end;
  const dx = end.x - start.x, dy = end.y - start.y;
  if (tool === 'arrow') {
    const length = Math.hypot(dx, dy);
    const angle = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
    return { x: start.x + Math.cos(angle) * length, y: start.y + Math.sin(angle) * length };
  }
  const size = Math.max(Math.abs(dx), Math.abs(dy));
  return { x: start.x + Math.sign(dx || 1) * size, y: start.y + Math.sign(dy || 1) * size };
}
