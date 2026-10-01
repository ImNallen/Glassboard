<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { DrawingHistory, OverlayRenderer, REGULAR_WIDTH, FREEHAND_TOOLS, CYCLE_COLORS, constrainEnd, createShape, cycleColor, measureText, shiftRainbow, shapeAtPoint, textFontSize, textLineHeight, type Shape, type Point } from './lib/drawing';
  import { activateOverlay, drawingEvents, reportHistory, type Session } from './lib/session';
  let { session, onerror, bounds, showGlow = true }: { session: Session; onerror: (error: unknown) => void; bounds?: { x: number; y: number; width: number; height: number }; showGlow?: boolean } = $props();
  let canvas: HTMLCanvasElement;
  let textarea = $state<HTMLTextAreaElement | undefined>();
  const history = new DrawingHistory();
  const renderer = new OverlayRenderer();
  let annotationSession = -1;
  let draft: Shape | null = null;
  let pointer: number | null = null;
  let lastPointer: Point | null = null;
  // Shapes swept by the eraser during the current drag; committed as one undo step on release.
  let erasing: Set<string> | null = null;
  let frame = 0;
  let fadeTimer: ReturnType<typeof setTimeout> | undefined;
  // The text tool edits inline; the canvas only draws the text once it is committed.
  let editor = $state<{ origin: Point; hue: number; text: string } | null>(null);
  const editorWidth = REGULAR_WIDTH;
  const editorFontSize = textFontSize(editorWidth);
  const editorLineHeight = textLineHeight(editorWidth);
  let editorColor = $derived(session.preferences.colorMode === 'cycle' ? cycleColor(session.cycleIndex) : session.preferences.color);
  let editorSize = $derived.by(() => {
    const ctx = editor && canvas?.getContext('2d');
    if (!editor || !ctx) return { width: 0, height: 0 };
    const size = measureText(ctx, editor.text || ' ', editorWidth);
    return { width: size.width + editorFontSize, height: Math.max(size.height, editorLineHeight) };
  });
  let editorGradient = $derived.by(() => {
    if (!editor || session.preferences.colorMode !== 'rainbow') return '';
    const start = Math.floor(editor.hue / 360 * CYCLE_COLORS.length);
    return `linear-gradient(90deg, ${CYCLE_COLORS.map((_, i) => cycleColor(start + i)).join(', ')})`;
  });
  function syncHistory(advanceCycle = false) {
    if (annotationSession < 0) return;
    reportHistory({ canUndo: history.canUndo, canRedo: history.canRedo }, annotationSession, advanceCycle).catch(onerror);
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
      const shapes = erasing?.size ? history.shapes.filter(shape => !erasing!.has(shape.id)) : history.shapes;
      if (ctx) renderer.paint(ctx, shapes, draft, width, height, scale, now);
      const next = history.nextFadeUpdate(now);
      if (next !== undefined) {
        if (next <= now) paint();
        else fadeTimer = setTimeout(paint, next - now);
      }
    });
  }
  function cancel() {
    if (pointer !== null && canvas?.hasPointerCapture(pointer)) canvas.releasePointerCapture(pointer);
    pointer = null; draft = null; lastPointer = null; erasing = null; paint();
  }
  /** Commit the stroke or erase in progress, as a pointer release would. */
  function finish() {
    if (erasing) {
      if (erasing.size && history.removeAll(erasing)) syncHistory();
      cancel();
      return;
    }
    if (draft) {
      const start = draft.points[0], end = draft.points[draft.points.length - 1];
      if (FREEHAND_TOOLS.includes(draft.tool) || Math.hypot(end.x - start.x, end.y - start.y) >= 3) {
        history.add(draft);
        syncHistory(draft.colorMode === 'cycle');
      }
    }
    cancel();
  }
  /** Finish pending text/strokes before exporting, without copying editor or control pixels. */
  export function exportShapes(): Shape[] {
    commitText(); finish();
    return structuredClone(history.shapes);
  }
  function blur() { commitText(); finish(); }
  $effect(() => { if (session.mode !== 'draw') blur(); });
  $effect(() => { if (session.preferences.tool !== 'text') commitText(); });
  function eraseAt(point: Point) {
    if (!erasing) return;
    const ctx = canvas.getContext('2d');
    const now = Date.now();
    const shapes = history.shapes.filter(shape => !erasing!.has(shape.id));
    const target = ctx && shapeAtPoint(ctx, shapes, point, now);
    if (target) { erasing.add(target.id); paint(); }
  }
  function openText(origin: Point) {
    commitText();
    editor = { origin, hue: Math.random() * 360, text: '' };
    tick().then(() => textarea?.focus());
  }
  let committing = false;
  function commitText() {
    const current = editor;
    if (!current || committing) return;
    committing = true;
    editor = null;
    const text = current.text.replace(/\s+$/, '');
    const ctx = canvas?.getContext('2d');
    if (text.trim() && ctx) {
      const shape = createShape(session.preferences, session.cycleIndex, current.origin);
      const size = measureText(ctx, text, shape.width);
      shape.hue = current.hue; shape.text = text;
      shape.points = [{ ...current.origin }, { x: current.origin.x + size.width, y: current.origin.y + size.height }];
      history.add(shape);
      syncHistory(shape.colorMode === 'cycle');
    }
    committing = false;
    paint();
  }
  function editorKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); editor = null; paint(); return; }
    if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); event.stopPropagation(); commitText(); }
  }
  function down(event: PointerEvent) {
    if (session.mode !== 'draw' || event.button !== 0 || pointer !== null) return;
    event.preventDefault();
    const point = { x: event.clientX, y: event.clientY };
    if (bounds && (point.x < bounds.x || point.y < bounds.y || point.x > bounds.x + bounds.width || point.y > bounds.y + bounds.height)) return;
    activateOverlay().catch(onerror);
    const tool = session.preferences.tool;
    if (tool === 'text') { openText(point); return; }
    commitText();
    pointer = event.pointerId; canvas.setPointerCapture(pointer);
    lastPointer = point;
    if (tool === 'eraser') { erasing = new Set(); eraseAt(point); return; }
    draft = createShape(session.preferences, session.cycleIndex, lastPointer);
    paint();
  }
  function move(event: PointerEvent) {
    if (event.pointerId !== pointer) return;
    // No button is held: the release happened where this page could not see it (outside
    // the window, or over another view). Keep what was drawn instead of waiting for a
    // pointerup that will not arrive, and leave the re-entry point out of the stroke.
    if (event.type === 'pointermove' && event.buttons === 0) { finish(); return; }
    const current = { x: event.clientX, y: event.clientY };
    if (erasing) {
      for (const sample of event.getCoalescedEvents?.() || [event]) eraseAt({ x: sample.clientX, y: sample.clientY });
      lastPointer = current;
      return;
    }
    if (!draft) return;
    if (lastPointer) shiftRainbow(draft, lastPointer, current);
    lastPointer = current;
    if (FREEHAND_TOOLS.includes(draft.tool)) {
      const samples = event.getCoalescedEvents?.() || [];
      for (const sample of samples.length ? samples : [event]) {
        const last = draft.points[draft.points.length - 1];
        if (Math.hypot(sample.clientX - last.x, sample.clientY - last.y) >= .6) draft.points.push({ x: sample.clientX, y: sample.clientY });
      }
    } else draft.points[1] = constrainEnd(draft.points[0], current, draft.tool, event.shiftKey);
    paint();
  }
  function up(event: PointerEvent) {
    if (event.pointerId !== pointer) return;
    move(event);
    finish();
  }
  // Pre-effects run during component initialization; all drawing state must exist first.
  // The generation travels with the session snapshot, so late-loading displays also reset.
  $effect.pre(() => {
    if (annotationSession === session.annotationSession) return;
    annotationSession = session.annotationSession;
    editor = null;
    cancel();
    history.reset();
    renderer.reset();
    // Hidden webviews can pause animation frames. Clear the backing bitmap now,
    // before it can be presented again when the native overlay is shown.
    cancelAnimationFrame(frame);
    frame = 0;
    clearTimeout(fadeTimer);
    canvas?.getContext('2d')?.clearRect(0, 0, canvas.width, canvas.height);
    syncHistory();
  });
  onMount(() => {
    let disposed = false, stop = () => {};
    drawingEvents(command => {
      commitText();
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
    return () => { disposed = true; stop(); cancelAnimationFrame(frame); clearTimeout(fadeTimer); renderer.reset(); };
  });
</script>
<svelte:window onresize={paint} onblur={blur} />
{#if session.mode === 'draw' && showGlow}
  <div class="annotation-glow" aria-hidden="true"></div>
{/if}
<canvas style:clip-path={bounds ? `inset(${bounds.y}px calc(100% - ${bounds.x + bounds.width}px) calc(100% - ${bounds.y + bounds.height}px) ${bounds.x}px)` : undefined} bind:this={canvas} class:concealed={session.mode === 'hidden'} class:text-tool={session.preferences.tool === 'text'} class:eraser-tool={session.preferences.tool === 'eraser'} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={cancel} onlostpointercapture={finish} oncontextmenu={event => event.preventDefault()} aria-label="Screen annotation canvas. Choose the eraser tool to remove drawings."></canvas>
{#if editor}
  <textarea bind:this={textarea} bind:value={() => editor?.text ?? '', value => { if (editor) editor.text = value; }} class="text-editor" class:rainbow={Boolean(editorGradient)} class:concealed={session.mode === 'hidden'}
    style:left={`${editor.origin.x}px`} style:top={`${editor.origin.y}px`} style:width={`${editorSize.width}px`} style:height={`${editorSize.height}px`}
    style:font-size={`${editorFontSize}px`} style:line-height={`${editorLineHeight}px`} style:color={editorGradient ? 'transparent' : editorColor} style:background-image={editorGradient || 'none'}
    rows="1" spellcheck="false" autocomplete="off" autocapitalize="off" aria-label="Annotation text. Enter commits, Shift+Enter adds a line, Escape cancels."
    onkeydown={editorKeydown} onblur={event => { if (event.target === textarea) commitText(); }} onpointerdown={event => event.stopPropagation()}></textarea>
{/if}
<style>
  .annotation-glow {
    position: fixed;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    box-shadow:
      inset 0 0 0 2px rgb(77 202 160 / .85),
      inset 0 0 12px 3px rgb(77 202 160 / .5),
      inset 0 0 32px 6px rgb(77 202 160 / .3);
    animation: annotation-glow-in 180ms ease-out both;
  }
  @keyframes annotation-glow-in { from { opacity: 0; } to { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .annotation-glow { animation: none; } }
  canvas { position: fixed; inset: 0; width: 100vw; height: 100vh; cursor: crosshair; touch-action: none; -webkit-user-select: none; user-select: none; }
  canvas.text-tool { cursor: text; }
  canvas.eraser-tool { cursor: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='22' height='22'%3E%3Ccircle cx='11' cy='11' r='8' fill='none' stroke='%23000' stroke-width='3'/%3E%3Ccircle cx='11' cy='11' r='8' fill='none' stroke='%23fff' stroke-width='1.5'/%3E%3C/svg%3E") 11 11, cell; }
  .concealed { visibility: hidden; }
  .text-editor { position: fixed; z-index: 2; margin: 0; padding: 0; border: 0; outline: 0; resize: none; overflow: hidden; background: transparent; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; font-weight: 600; white-space: pre; caret-color: #3564c5; text-shadow: 0 1px 3px rgba(25, 30, 40, .18); -webkit-user-select: text; user-select: text; }
  .text-editor.rainbow { -webkit-background-clip: text; background-clip: text; text-shadow: none; }
</style>
