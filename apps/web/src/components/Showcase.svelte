<script lang="ts">
  import { onMount } from 'svelte';
  import { createShape, render, FADE_MS, type Point, type Shape, type Tool } from '@glassboard/ui/drawing';

  // A looping demonstration drawn with the real engine over the mock window behind it:
  // circle the card, point at it, highlight a line, hold, then let it fade like auto-fade would.
  type Step = { tool: Tool; from: Point; to: Point; start: number; duration: number };
  // Positions are fractions of the window; times are milliseconds into a loop.
  const STEPS: Step[] = [
    { tool: 'ellipse', from: { x: .56, y: .26 }, to: { x: .93, y: .66 }, start: 500, duration: 800 },
    { tool: 'arrow', from: { x: .16, y: .84 }, to: { x: .53, y: .52 }, start: 1500, duration: 550 },
    { tool: 'highlighter', from: { x: .11, y: .31 }, to: { x: .46, y: .31 }, start: 2250, duration: 650 },
  ];
  const HOLD_UNTIL = 4600, LOOP = 6200, SAMPLES = 48;
  const style = { tool: 'arrow' as Tool, color: '#f46b78', colorMode: 'rainbow' as const, autoFadeSeconds: 0 as const };

  let host: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let glowing = $state(false);

  const ease = (t: number) => t <= 0 ? 0 : t >= 1 ? 1 : 1 - Math.pow(1 - t, 3);
  const lerp = (a: Point, b: Point, t: number): Point => ({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t });

  function newLoop(): Shape[] {
    return STEPS.map(step => createShape({ ...style, tool: step.tool }, 0, step.from));
  }

  /** Lay out the loop's shapes at `elapsed` ms, in CSS pixels for a window of the given size. */
  function layout(shapes: Shape[], elapsed: number, width: number, height: number, expiresAt?: number): Shape[] {
    const px = (p: Point): Point => ({ x: p.x * width, y: p.y * height });
    const out: Shape[] = [];
    STEPS.forEach((step, i) => {
      const progress = ease((elapsed - step.start) / step.duration);
      if (progress <= 0) return;
      const from = px(step.from), to = px(step.to);
      const shape = shapes[i];
      const travelled = Math.hypot(to.x - from.x, to.y - from.y) * progress;
      let points: Point[];
      if (step.tool === 'highlighter') {
        const count = Math.max(2, Math.ceil(SAMPLES * progress));
        points = Array.from({ length: count }, (_, k) => {
          const t = k / (SAMPLES - 1);
          const p = lerp(from, to, t);
          return { x: p.x, y: p.y + Math.sin(t * Math.PI * 3) * height * .006 };
        });
      } else {
        points = [from, lerp(from, to, progress)];
      }
      // The hue drifts with distance dragged, as it does under a real pointer.
      out.push({ ...shape, points, hue: ((shape.hue ?? 0) + travelled * .6) % 360, expiresAt });
    });
    return out;
  }

  onMount(() => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
    let width = 0, height = 0, scale = 1;
    let shapes = newLoop();
    let loopStart = performance.now();
    let frame = 0;

    function fit() {
      const rect = host.getBoundingClientRect();
      scale = window.devicePixelRatio || 1;
      width = rect.width; height = rect.height;
      canvas.width = Math.round(width * scale); canvas.height = Math.round(height * scale);
    }
    function still() {
      glowing = true;
      render(ctx!, layout(shapes, HOLD_UNTIL - 1, width, height), null, width, height, scale);
    }
    function tick(now: number) {
      let elapsed = now - loopStart;
      if (elapsed >= LOOP) { loopStart = now; elapsed = 0; shapes = newLoop(); }
      const fadeEnd = loopStart + HOLD_UNTIL + FADE_MS;
      glowing = elapsed < HOLD_UNTIL + FADE_MS;
      render(ctx!, layout(shapes, elapsed, width, height, fadeEnd), null, width, height, scale, now);
      frame = requestAnimationFrame(tick);
    }

    const observer = new ResizeObserver(() => { fit(); if (reduced) still(); });
    observer.observe(host);
    fit();
    if (reduced) still(); else frame = requestAnimationFrame(tick);
    return () => { observer.disconnect(); cancelAnimationFrame(frame); };
  });
</script>

<div bind:this={host} class="showcase" aria-hidden="true">
  <div class="glow" class:on={glowing}></div>
  <canvas bind:this={canvas}></canvas>
</div>

<style>
  /* border-radius does not inherit through the island wrapper, so the window passes its radius as a variable. */
  .showcase { position: absolute; inset: 0; border-radius: calc(var(--radius, 14px) - 1px); pointer-events: none; }
  canvas { position: absolute; inset: 0; width: 100%; height: 100%; }
  /* The app's annotation-mode glow, scaled to the small window. */
  .glow { position: absolute; inset: 0; border-radius: inherit; overflow: hidden; opacity: 0; transition: opacity .25s ease; box-shadow: inset 0 0 0 2px rgb(77 202 160 / .85), inset 0 0 8px 2px rgb(77 202 160 / .5), inset 0 0 22px 5px rgb(77 202 160 / .3); }
  .glow.on { opacity: 1; }
  @media (prefers-reduced-motion: reduce) { .glow { transition: none; } }
</style>
