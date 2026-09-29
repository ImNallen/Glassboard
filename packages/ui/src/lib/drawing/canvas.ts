import { cycleColor, CYCLE_COLORS, rainbowAxis, shapeOpacity, type Point, type Shape } from './shapes';

export const TEXT_FONT_FAMILY = "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
const TEXT_LINE_HEIGHT = 1.25;
/** Match text to the shape geometry; Regular width produces 24 px text. */
export function textFontSize(width: number): number { return Math.round(12 + width * 3); }
export function textLineHeight(width: number): number { return Math.round(textFontSize(width) * TEXT_LINE_HEIGHT); }
export function textFont(width: number): string { return `600 ${textFontSize(width)}px ${TEXT_FONT_FAMILY}`; }
/** Measure multi-line text the way `render` lays it out. */
export function measureText(ctx: CanvasRenderingContext2D, text: string, width: number): { width: number; height: number } {
  const lines = text.split('\n');
  ctx.save();
  ctx.font = textFont(width);
  const widest = Math.max(0, ...lines.map(line => ctx.measureText(line).width));
  ctx.restore();
  return { width: Math.ceil(widest), height: lines.length * textLineHeight(width) };
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
  } else if (shape.tool === 'text') {
    ctx.rect(Math.min(start.x, end.x), Math.min(start.y, end.y), Math.abs(end.x - start.x), Math.abs(end.y - start.y));
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
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    for (let i = shapes.length - 1; i >= 0; i--) {
      const shape = shapes[i];
      if (!shape.points.length || shapeOpacity(shape, now) === 0) continue;
      path(ctx, shape);
      ctx.lineWidth = (shape.tool === 'arrow' ? 0 : shape.width * (shape.tool === 'highlighter' ? 5 : 1)) + 12;
      const filled = shape.tool === 'arrow' || shape.tool === 'text';
      if ((filled && ctx.isPointInPath(point.x, point.y)) || ctx.isPointInStroke(point.x, point.y)) return shape;
    }
  } finally {
    ctx.restore();
  }
}

function applyShadow(ctx: CanvasRenderingContext2D, scale: number) {
  // Canvas shadows use device pixels independently of the drawing transform.
  ctx.shadowColor = 'rgba(25, 30, 40, .18)';
  ctx.shadowBlur = 3 * scale;
  ctx.shadowOffsetY = scale;
}

export function render(ctx: CanvasRenderingContext2D, shapes: Shape[], draft: Shape | null, width: number, height: number, scale: number, now = Date.now()) {
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.clearRect(0, 0, width, height);
  for (const shape of draft ? [...shapes, draft] : shapes) {
    const opacity = shapeOpacity(shape, now);
    if (opacity === 0) continue;
    ctx.save();
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    path(ctx, shape);
    const color = strokeColor(ctx, shape);
    if (shape.tool === 'text') {
      const origin = shape.points[0], size = textFontSize(shape.width), lineHeight = textLineHeight(shape.width);
      const lines = (shape.text ?? '').split('\n');
      // Center each line in its line box so the canvas matches the inline editor.
      const lineY = (i: number) => origin.y + i * lineHeight + (lineHeight - size) / 2;
      ctx.font = textFont(shape.width);
      ctx.textBaseline = 'top';
      ctx.strokeStyle = color;
      ctx.globalAlpha = .055 * opacity;
      for (let spread = 6; spread >= 1; spread--) {
        ctx.lineWidth = spread;
        lines.forEach((line, i) => ctx.strokeText(line, origin.x, lineY(i)));
      }
      ctx.globalAlpha = opacity;
      applyShadow(ctx, scale);
      ctx.fillStyle = color;
      lines.forEach((line, i) => ctx.fillText(line, origin.x, lineY(i)));
    } else if (shape.tool === 'highlighter') {
      ctx.globalAlpha = .35 * opacity;
      ctx.lineWidth = shape.width * 5;
      ctx.strokeStyle = color;
      ctx.stroke();
    } else {
      // Feather the same stroke color up to 3 px outward, including every rainbow segment.
      ctx.strokeStyle = color;
      ctx.globalAlpha = .055 * opacity;
      for (let spread = 6; spread >= 1; spread--) {
        ctx.lineWidth = (shape.tool === 'arrow' ? 0 : shape.width) + spread;
        ctx.stroke();
      }
      ctx.globalAlpha = opacity;
      applyShadow(ctx, scale);
      if (shape.tool === 'arrow') {
        ctx.fillStyle = color;
        ctx.fill();
      } else {
        ctx.lineWidth = shape.width;
        ctx.strokeStyle = color;
        ctx.stroke();
      }
    }
    ctx.restore();
  }
}
