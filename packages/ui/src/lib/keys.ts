import { colorShortcut, toolShortcut } from './shortcuts';
import { native, type Preferences, type Session } from './session';

type Handlers = {
  run: (action: string) => void;
  save: (preferences: Preferences) => void;
  /** Start screenshot capture when the host provides a capture editor. */
  capture?: () => void;
  /** Mirror the default show/hide binding in the browser. Defaults to on outside Tauri, where the native binding lives in Rust. */
  toggleShortcut?: boolean;
};

/**
 * Routes a keydown event to drawing shortcuts: show/hide (browser only, optional),
 * Escape, capture, undo/redo, colors, and tools. Keys typed into form controls are left alone.
 */
export function drawingKeydown(event: KeyboardEvent, session: Session, { run, save, capture, toggleShortcut = !native }: Handlers) {
  if (event.defaultPrevented) return;
  if ((event.target as HTMLElement)?.closest('input, textarea, select, [contenteditable="true"]')) return;
  const key = event.key.toLowerCase();
  const command = event.metaKey || event.ctrlKey;
  if (toggleShortcut && command && event.shiftKey && key === 'a') { event.preventDefault(); run('toggle'); return; }
  if (session.mode === 'hidden') return;
  if (capture && command && !event.shiftKey && !event.altKey && key === 's') {
    event.preventDefault();
    if (!event.repeat) capture();
    return;
  }
  if (key === 'escape') { event.preventDefault(); run('hide'); return; }
  if (command && key === 'z') { event.preventDefault(); run(event.shiftKey ? 'redo' : 'undo'); return; }
  const color = colorShortcut(event);
  if (color) { event.preventDefault(); save({ ...session.preferences, ...color }); return; }
  const tool = toolShortcut(event);
  if (tool) { event.preventDefault(); save({ ...session.preferences, tool }); }
}
