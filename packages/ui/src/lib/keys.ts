import { COLOR_SHORTCUTS, colorChoice, commandFor, matchesShortcut, TOOL_SHORTCUTS } from './shortcuts';
import { mac, native, type Preferences, type Session } from './session';

type Handlers = {
  run: (action: string) => void;
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
  if ((event.target as HTMLElement)?.closest('input, textarea, select, [contenteditable="true"]')) return;
  const { keybindings, shortcut } = session.preferences;
  if (toggleShortcut && matchesShortcut(event, shortcut, mac)) { event.preventDefault(); run('toggle'); return; }
  if (session.mode === 'hidden') return;
  const command = commandFor(event, keybindings, mac);
  if (command === 'capture') {
    if (!capture) return;
    event.preventDefault();
    if (!event.repeat) capture();
    return;
  }
  if (command === 'hide' || command === 'undo' || command === 'redo') { event.preventDefault(); run(command); return; }
  const color = COLOR_SHORTCUTS.find(choice => choice.command === command);
  if (color) {
    event.preventDefault();
    save({ ...session.preferences, ...colorChoice(color, session.preferences.swatches) });
    return;
  }
  const tool = TOOL_SHORTCUTS.find(tool => tool.command === command);
  if (tool) { event.preventDefault(); save({ ...session.preferences, tool: tool.id }); }
}
