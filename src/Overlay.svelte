<script lang="ts">
  import { onMount } from 'svelte';
  import { DrawingHistory, constrainEnd, createShape, shiftRainbow, shapeAtPoint, render, type Shape, type Point } from './lib/drawing';
  import { activateOverlay, drawingEvents, reportHistory, type Session } from './lib/session';
  let { session, onerror }: { session: Session; onerror: (error: unknown) => void } = $props();
  let canvas: HTMLCanvasElement;
  const history = new DrawingHistory();
  let draft: Shape | null = null;
  let pointer: number | null = null;
  let lastPointer: Point | null = null;
  let hover: Point | null = null;
  let frame = 0;
  let fadeTimer: ReturnType<typeof setTimeout> | undefined;
  function syncHistory(advanceCycle = false) {
    reportHistory({ canUndo: history.canUndo, canRedo: history.canRedo }, advanceCycle).catch(onerror);
  }
  function paint() {
    clearTimeout(fadeTimer);
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      if (!canvas) return;
      const now = Date.now();
      if (history.expire(now)) syncHistory();
      const scale = window.devicePixelRatio || 1;
      const width = window.innerWidth, height = window.innerHeight;
      if (canvas.width !== Math.round(width * scale) || canvas.height !== Math.round(height * scale)) {
        canvas.width = Math.round(width * scale); canvas.height = Math.round(height * scale);
      }
      const ctx = canvas.getContext('2d');
      if (ctx) render(ctx, history.shapes, draft, width, height, scale, now);
      const next = history.nextFadeUpdate(now);
      if (next !== undefined) {
        if (next <= now) paint();
        else fadeTimer = setTimeout(paint, next - now);
      }
    });
  }
  function cancel() {
    if (pointer !== null && canvas?.hasPointerCapture(pointer)) canvas.releasePointerCapture(pointer);
    pointer = null; draft = null; lastPointer = null; paint();
  }
  function leave() { hover = null; }
  function blur() { leave(); cancel(); }
  $effect(() => { if (session.mode !== 'draw') blur(); });
  function down(event: PointerEvent) {
    if (session.mode !== 'draw' || event.button !== 0 || pointer !== null) return;
    event.preventDefault();
    hover = { x: event.clientX, y: event.clientY };
    activateOverlay().catch(onerror);
    pointer = event.pointerId; canvas.setPointerCapture(pointer);
    lastPointer = { x: event.clientX, y: event.clientY };
    draft = createShape(session.preferences, session.cycleIndex, lastPointer);
    paint();
  }
  function move(event: PointerEvent) {
    hover = { x: event.clientX, y: event.clientY };
    if (!draft || event.pointerId !== pointer) return;
    const current = { x: event.clientX, y: event.clientY };
    if (lastPointer) shiftRainbow(draft, lastPointer, current);
    lastPointer = current;
    if (draft.tool === 'highlighter') {
      const samples = event.getCoalescedEvents?.() || [];
      for (const sample of samples.length ? samples : [event]) {
        const last = draft.points[draft.points.length - 1];
        if (Math.hypot(sample.clientX - last.x, sample.clientY - last.y) >= .6) draft.points.push({ x: sample.clientX, y: sample.clientY });
      }
    } else draft.points[1] = constrainEnd(draft.points[0], { x: event.clientX, y: event.clientY }, draft.tool, event.shiftKey);
    paint();
  }
  function up(event: PointerEvent) {
    if (!draft || event.pointerId !== pointer) return;
    move(event);
    const start = draft.points[0], end = draft.points[draft.points.length - 1];
    if (draft.tool === 'highlighter' || Math.hypot(end.x - start.x, end.y - start.y) >= 3) {
      history.add(draft);
      syncHistory(draft.colorMode === 'cycle');
    }
    cancel();
  }
  onMount(() => {
    let disposed = false, stop = () => {};
    drawingEvents(command => {
      if (command === 'erase') {
        if (session.mode !== 'draw' || !hover || draft) return;
        const now = Date.now();
        const expired = history.expire(now);
        const ctx = canvas?.getContext('2d');
        const target = ctx && shapeAtPoint(ctx, history.shapes, hover, now);
        if (target && history.remove(target.id)) {
          activateOverlay().catch(onerror);
          syncHistory();
        } else if (expired) syncHistory();
        paint();
        return;
      }
      cancel();
      history.expire(Date.now());
      if (command === 'undo') history.undo();
      if (command === 'redo') history.redo();
      if (command === 'clear') history.clear();
      syncHistory();
      paint();
    }).then(fn => { if (disposed) fn(); else stop = fn; }).catch(onerror);
    syncHistory();
    paint();
    return () => { disposed = true; stop(); cancelAnimationFrame(frame); clearTimeout(fadeTimer); };
  });
</script>
<svelte:window onresize={paint} onblur={blur} />
<canvas bind:this={canvas} class:passthrough={session.mode !== 'draw'} class:concealed={session.mode === 'hidden'} onpointerdown={down} onpointermove={move} onpointerleave={leave} onpointerup={up} onpointercancel={cancel} onlostpointercapture={cancel} oncontextmenu={event => event.preventDefault()} aria-label="Screen annotation canvas. Hover over a drawing and press X to erase it."></canvas>
<style>
  canvas { position: fixed; inset: 0; width: 100vw; height: 100vh; cursor: crosshair; touch-action: none; -webkit-user-select: none; user-select: none; }
  .passthrough { pointer-events: none; }
  .concealed { visibility: hidden; }
</style>
