<script lang="ts">
  import { untrack } from 'svelte';
  import { hexToHsv, hsvToHex, parseHexColor, sameColor, type Hsv } from '@glassboard/ui/swatches';
  /**
   * An inline color picker. The OS color panel would take focus and close the settings window,
   * so saturation, brightness, and hue are chosen here. `oninput` follows a drag; `onchange` commits.
   */
  let { value, label, oninput, onchange }: { value: string; label: string; oninput: (color: string) => void; onchange: (color: string) => void } = $props();
  const initial = (() => value)();
  let hsv = $state<Hsv>(hexToHsv(initial));
  let hex = $state(initial.toUpperCase());
  let dragging = $state(false);
  let field: HTMLDivElement;
  let current = $derived(hsvToHex(hsv));
  // Follow outside changes, keeping the hue for grays where the color alone can't say it.
  $effect(() => {
    const next = value;
    untrack(() => {
      if (dragging || sameColor(next, current)) return;
      const parsed = hexToHsv(next);
      hsv = parsed.s && parsed.v ? parsed : { ...parsed, h: hsv.h };
      hex = next.toUpperCase();
    });
  });
  function set(next: Partial<Hsv>, commit: boolean) {
    hsv = { ...hsv, ...next };
    hex = current.toUpperCase();
    (commit ? onchange : oninput)(current);
  }
  function pick(event: PointerEvent, commit = false) {
    const rect = field.getBoundingClientRect();
    const clamp = (value: number) => Math.max(0, Math.min(1, value));
    set({ s: clamp((event.clientX - rect.left) / rect.width), v: clamp(1 - (event.clientY - rect.top) / rect.height) }, commit);
  }
  function fieldKeydown(event: KeyboardEvent) {
    const step = event.shiftKey ? .1 : .02;
    const moves: Record<string, Partial<Hsv>> = {
      ArrowLeft: { s: Math.max(0, hsv.s - step) }, ArrowRight: { s: Math.min(1, hsv.s + step) },
      ArrowDown: { v: Math.max(0, hsv.v - step) }, ArrowUp: { v: Math.min(1, hsv.v + step) },
    };
    if (!moves[event.key]) return;
    event.preventDefault();
    set(moves[event.key], true);
  }
  function commitHex() {
    const parsed = parseHexColor(hex);
    if (!parsed) { hex = current.toUpperCase(); return; }
    const next = hexToHsv(parsed);
    hsv = next.s && next.v ? next : { ...next, h: hsv.h };
    hex = parsed.toUpperCase();
    onchange(parsed);
  }
</script>

<div class="picker">
  <div bind:this={field} class="field" style:--hue={`hsl(${hsv.h} 100% 50%)`} role="slider" tabindex="0"
    aria-label={`${label} saturation and brightness`} aria-valuetext={`Saturation ${Math.round(hsv.s * 100)}%, brightness ${Math.round(hsv.v * 100)}%`}
    aria-valuenow={Math.round(hsv.s * 100)} aria-valuemin={0} aria-valuemax={100}
    onpointerdown={event => { dragging = true; field.setPointerCapture(event.pointerId); pick(event); }}
    onpointermove={event => { if (dragging) pick(event); }}
    onpointerup={event => { if (!dragging) return; dragging = false; pick(event, true); }}
    onpointercancel={() => { dragging = false; onchange(current); }}
    onkeydown={fieldKeydown}>
    <span class="thumb" style:left={`${hsv.s * 100}%`} style:top={`${(1 - hsv.v) * 100}%`} style:background={current}></span>
  </div>
  <input class="hue" type="range" min="0" max="359" step="1" aria-label={`${label} hue`} value={Math.round(hsv.h)}
    oninput={event => set({ h: Number(event.currentTarget.value) }, false)} onchange={() => onchange(current)}/>
  <label class="hex">
    <span class="preview" style:background={current} aria-hidden="true"></span>
    <span class="hex-label">Hex</span>
    <input type="text" spellcheck="false" autocomplete="off" maxlength="7" aria-label={`${label} hex code`} bind:value={hex}
      onkeydown={event => { if (event.key === 'Enter') { event.preventDefault(); commitHex(); } }} onblur={commitHex}/>
  </label>
</div>

<style>
  .picker { display: flex; flex-direction: column; gap: 10px; }
  .field { position: relative; height: 104px; border-radius: 8px; cursor: crosshair; touch-action: none; box-shadow: inset 0 0 0 1px var(--swatch-border);
    background: linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, var(--hue)); }
  .field:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .thumb { position: absolute; width: 14px; height: 14px; border-radius: 50%; translate: -50% -50%; pointer-events: none; box-shadow: 0 0 0 2px #fff, 0 0 0 3px #0000004d, 0 1px 3px #00000059; }
  .hue { -webkit-appearance: none; appearance: none; width: 100%; height: 12px; margin: 0; border-radius: 6px; cursor: pointer; box-shadow: inset 0 0 0 1px var(--swatch-border);
    background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00); }
  .hue::-webkit-slider-thumb { -webkit-appearance: none; appearance: none; width: 14px; height: 14px; border-radius: 50%; background: #fff; box-shadow: 0 0 0 1px #0000004d, 0 1px 3px #00000059; }
  .hue::-moz-range-thumb { width: 14px; height: 14px; border: 0; border-radius: 50%; background: #fff; box-shadow: 0 0 0 1px #0000004d, 0 1px 3px #00000059; }
  .hue:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 3px; }
  .hex { display: flex; align-items: center; gap: 8px; height: 30px; padding: 0 4px 0 6px; border-radius: 8px; border: 1px solid var(--border); background: var(--input-surface); }
  .hex:focus-within { border-color: var(--focus-ring); box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus-ring) 22%, transparent); }
  .preview { width: 18px; height: 18px; flex-shrink: 0; border-radius: 5px; box-shadow: inset 0 0 0 1px var(--swatch-border); }
  .hex-label { font-size: 10px; font-weight: 600; letter-spacing: .6px; text-transform: uppercase; color: var(--muted); }
  .hex input { flex: 1; min-width: 0; height: 100%; border: 0; outline: 0; background: none; color: var(--text); font-size: 12px; font-variant-numeric: tabular-nums; letter-spacing: .3px; -webkit-user-select: text; user-select: text; }
</style>
