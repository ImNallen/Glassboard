import { colorShortcut, commandFor, matchesShortcut, platformMac, toolShortcut } from './shortcuts';
import { isEditableTarget } from './selection';
import { native, type Action, type Preferences, type Session } from './session';

type Handlers = {
  run: (action: Action) => void;
  save: (preferences: Preferences) => void;
  /** Start screenshot capture when the host provides a capture editor. */
  capture?: () => void;
  /** Mirror the show/hide shortcut in the browser. Defaults to on outside Tauri, where the native binding lives in Rust. */
  toggleShortcut?: boolean;
};

/**
 * Routes a keydown event to the user's bindings: show/hide (browser only, optional),
 * return to work, capture, undo/redo, colors, and tools. Keys typed into form controls are left alone.
 */
export function drawingKeydown(event: KeyboardEvent, session: Session, { run, save, capture, toggleShortcut = !native }: Handlers) {
  if (event.defaultPrevented) return;
  if (isEditableTarget(event.target)) return;
  const { keybindings, shortcut } = session.preferences;
  if (toggleShortcut && matchesShortcut(event, shortcut, platformMac)) { event.preventDefault(); run('toggle'); return; }
  if (session.mode === 'hidden') return;
  const command = commandFor(event, keybindings, platformMac);
  if (command === 'capture') {
    if (!capture) return;
    event.preventDefault();
    if (!event.repeat) capture();
    return;
  }
  if (command === 'hide' || command === 'undo' || command === 'redo') { event.preventDefault(); run(command); return; }
  const color = colorShortcut(event, keybindings, platformMac, session.preferences.swatches);
  if (color) { event.preventDefault(); save({ ...session.preferences, ...color }); return; }
  const tool = toolShortcut(event, keybindings, platformMac);
  if (tool) { event.preventDefault(); save({ ...session.preferences, tool }); }
}
