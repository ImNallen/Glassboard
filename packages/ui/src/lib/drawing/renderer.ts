import { render } from './canvas';
import { FADE_MS, shapeOpacity, type Shape } from './shapes';

/** Rasterize the stable prefix once, then paint fading shapes and the draft above it. */
export class OverlayRenderer {
  private cache: HTMLCanvasElement | undefined;
  private shapes: Shape[] | undefined;
  private prefix = 0;
  private fadeAt = Infinity;
  private width = 0;
  private height = 0;
  private scale = 0;

  constructor(private createCanvas = () => document.createElement('canvas')) {}

  reset() {
    this.shapes = undefined;
    this.prefix = 0;
    this.fadeAt = Infinity;
    // Hidden displays should not retain another full-resolution bitmap.
    if (this.cache) { this.cache.width = 0; this.cache.height = 0; }
  }

  paint(ctx: CanvasRenderingContext2D, shapes: Shape[], draft: Shape | null, width: number, height: number, scale: number, now = Date.now()) {
    const resized = width !== this.width || height !== this.height || scale !== this.scale;
    const changed = shapes !== this.shapes;
    let prefix = this.prefix;
    if (changed || now >= this.fadeAt) {
      prefix = 0;
      this.fadeAt = Infinity;
      // Only a prefix can be cached: a newer permanent shape must remain above
      // an older fading shape, including translucent highlighter strokes.
      for (const shape of shapes) {
        if (shapeOpacity(shape, now) !== 1) break;
        prefix++;
        if (shape.expiresAt !== undefined) this.fadeAt = Math.min(this.fadeAt, shape.expiresAt - FADE_MS);
      }
    }
    if (!prefix) {
      this.reset();
      render(ctx, shapes, draft, width, height, scale, now);
      return;
    }
    this.cache ??= this.createCanvas();
    const cached = this.cache.getContext('2d');
    if (!cached) { render(ctx, shapes, draft, width, height, scale, now); return; }
    if (changed || resized || prefix !== this.prefix) {
      this.cache.width = Math.round(width * scale);
      this.cache.height = Math.round(height * scale);
      render(cached, shapes.slice(0, prefix), null, width, height, scale, now);
    }
    this.shapes = shapes; this.prefix = prefix;
    this.width = width; this.height = height; this.scale = scale;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
    ctx.drawImage(this.cache, 0, 0);
    if (prefix < shapes.length || draft) render(ctx, shapes.slice(prefix), draft, width, height, scale, now, false);
  }
}
