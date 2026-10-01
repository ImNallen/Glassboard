import { tick, untrack } from 'svelte';
import type { ToolbarPosition } from './session';

type Hint = { label: string; key?: string; description?: string };
type Options = {
  readonly drawing: boolean;
  readonly expanded: boolean;
  readonly placement: ToolbarPosition;
  readonly host: HTMLDivElement | undefined;
  readonly tooltip: HTMLDivElement | undefined;
};

/** Shared tooltip state for DOM hover, keyboard focus, and native cursor samples. */
export function createToolbarHints(options: Options) {
  let tip = $state<(Hint & { node: HTMLButtonElement }) | null>(null);
  let offset = $state(0);
  let ready = $state(false);
  let pendingTip: HTMLButtonElement | null = null;
  let tipTimer: ReturnType<typeof setTimeout>;
  let hideTimer: ReturnType<typeof setTimeout>;
  const hints = new Map<HTMLButtonElement, { enter: () => void; leave: () => void }>();
  let hoveredHint: HTMLButtonElement | null = null;

  function hover(node: HTMLButtonElement | null) {
    if (hoveredHint === node) return;
    if (hoveredHint) hints.get(hoveredHint)?.leave();
    hoveredHint = node;
    if (node) hints.get(node)?.enter();
  }
  function blur() {
    hover(null);
    dismiss();
  }

  function dismiss() {
    clearTimeout(tipTimer);
    clearTimeout(hideTimer);
    pendingTip = null;
    tip = null;
    ready = false;
  }

  function hideSoon() {
    clearTimeout(tipTimer);
    pendingTip = null;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(dismiss, 100);
  }

  function position() {
    const { host, tooltip, placement } = options;
    if (!tip || !tooltip || !host) return;
    const vertical = placement !== 'bottom';
    const anchor = tip.node.getBoundingClientRect();
    const bounds = host.getBoundingClientRect();
    const size = vertical ? tooltip.offsetHeight : tooltip.offsetWidth;
    const center = vertical
      ? (anchor.top + anchor.bottom) / 2 - bounds.top
      : (anchor.left + anchor.right) / 2 - bounds.left;
    const available = vertical ? bounds.height : bounds.width;
    offset = Math.max(0, Math.min(center - size / 2, available - size));
    ready = true;
  }

  // One shared tooltip lives outside the toolbar's scrolling/clipping container.
  function hint(node: HTMLButtonElement, content: Hint) {
    function show(immediate = false) {
      clearTimeout(tipTimer);
      clearTimeout(hideTimer);
      pendingTip = node;
      const reveal = () => {
        if (pendingTip !== node || !options.drawing) return;
        ready = false;
        tip = { ...content, node };
      };
      if (immediate || tip) reveal();
      else tipTimer = setTimeout(reveal, 180);
    }
    const enter = () => hover(node);
    const focus = () => { if (node.matches(':focus-visible')) show(true); };
    const leave = () => { if (pendingTip === node || tip?.node === node) hideSoon(); };
    const pointerLeave = () => { if (hoveredHint === node) hover(null); };
    hints.set(node, { enter: () => show(), leave });
    node.addEventListener('pointerenter', enter);
    node.addEventListener('pointerleave', pointerLeave);
    node.addEventListener('focus', focus);
    node.addEventListener('blur', leave);
    node.addEventListener('click', dismiss);
    return {
      update(next: Hint) {
        content = next;
        // Svelte calls update inside the effect tracking the hint's parameters; reading tip there would re-run it on every tip change.
        untrack(() => { if (tip?.node === node) tip = { ...next, node }; });
      },
      destroy() {
        node.removeEventListener('pointerenter', enter);
        node.removeEventListener('pointerleave', pointerLeave);
        node.removeEventListener('focus', focus);
        node.removeEventListener('blur', leave);
        node.removeEventListener('click', dismiss);
        if (hoveredHint === node) hover(null);
        hints.delete(node);
        if (pendingTip === node || tip?.node === node) dismiss();
      },
    };
  }

  $effect(() => {
    const node = tip?.node;
    node?.setAttribute('aria-describedby', 'toolbar-tooltip');
    return () => node?.removeAttribute('aria-describedby');
  });
  $effect(() => {
    if (!options.drawing || !options.expanded) blur();
  });
  $effect(() => {
    options.placement;
    if (tip) tick().then(position);
  });

  return {
    get tip() { return tip; },
    get offset() { return offset; },
    get ready() { return ready; },
    hint,
    dismiss,
    hideSoon,
    position,
    blur,
    keepOpen() { clearTimeout(hideTimer); },
    hoverTarget(target: Element | null) {
      const button = target?.closest('button') as HTMLButtonElement | null;
      hover(button && hints.has(button) ? button : null);
      if (target && options.tooltip?.contains(target)) clearTimeout(hideTimer);
    },
  };
}
