import { colorShortcut, toolShortcut } from './shortcuts';
import { native, type Preferences, type Session } from './session';

type Handlers = {
  run: (action: string) => void;
  save: (preferences: Preferences) => void;
  /** Mirror the default show/hide binding in the browser. Defaults to on outside Tauri, where the native binding lives in Rust. */
  toggleShortcut?: boolean;
};

/**
 * Routes a keydown event to drawing shortcuts: show/hide (browser only, optional),
 * Escape, undo/redo, colors, and tools. Keys typed into form controls are left alone.
 */
export function drawingKeydown(event: KeyboardEvent, session: Session, { run, save, toggleShortcut = !native }: Handlers) {
  if ((event.target as HTMLElement)?.closest('input, textarea, select, [contenteditable="true"]')) return;
  const key = event.key.toLowerCase();
  const command = event.metaKey || event.ctrlKey;
  if (toggleShortcut && command && event.shiftKey && key === 'a') { event.preventDefault(); run('toggle'); return; }
  if (session.mode === 'hidden') return;
  if (key === 'escape') { event.preventDefault(); run('hide'); return; }
  if (command && key === 'z') { event.preventDefault(); run(event.shiftKey ? 'redo' : 'undo'); return; }
  const color = colorShortcut(event);
  if (color) { event.preventDefault(); save({ ...session.preferences, ...color }); return; }
  const tool = toolShortcut(event);
  if (tool) { event.preventDefault(); save({ ...session.preferences, tool }); }
}
