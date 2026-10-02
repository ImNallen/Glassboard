import type { Session } from '@glassboard/ui/session';

/** The `?surface=` each native window loads, fixed by `Surface::url` in src-tauri/src/windows/mod.rs. */
const SURFACES = ['overlay', 'capture', 'toolbar', 'settings', 'tutorial'] as const;
export type Surface = (typeof SURFACES)[number];
/** What a window can mount: one of the surfaces, or the browser preview's landing page behind them. */
export type Part = Surface | 'backdrop';

export function parseSurface(search: string): Surface | null {
  const value = new URLSearchParams(search).get('surface');
  return SURFACES.find(surface => surface === value) ?? null;
}

/** A native window mounts the one surface Rust opened it for. The browser preview mounts every drawing surface on one page unless a `?surface=` picks one, and always hosts the capture editor since no capture window exists. An open capture replaces the drawing surfaces, and its editor waits for the pixels. */
export function windowParts(surface: Surface | null, native: boolean, capture: Session['capture']): Part[] {
  const hosted: Part[] = native ? (surface ? [surface] : []) : surface ? [...new Set<Part>([surface, 'capture'])] : ['backdrop', 'overlay', 'toolbar', 'tutorial', 'capture'];
  return hosted.filter(part => part === 'settings' || (part === 'capture' ? Boolean(capture?.ready) : !capture));
}
