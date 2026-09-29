import { CYCLE_COLORS, type ColorMode, type Tool } from './drawing';

export const DEFAULT_SHORTCUT = 'CommandOrControl+Shift+A';
type KeyEvent = Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>;

export const TOOL_SHORTCUTS: readonly { id: Tool; name: string; key: string }[] = [
  { id: 'arrow', name: 'Arrow', key: '1' },
  { id: 'pen', name: 'Pen', key: '2' },
  { id: 'rectangle', name: 'Square', key: '3' },
  { id: 'ellipse', name: 'Circle', key: '4' },
  { id: 'eraser', name: 'Eraser', key: 'E' },
  { id: 'text', name: 'Text', key: 'T' },
  { id: 'highlighter', name: 'Highlighter', key: 'H' },
];

export const COLOR_SHORTCUTS: readonly { key: string; name: string; colorMode: ColorMode; color?: string }[] = [
  { key: '1', name: 'Rainbow', colorMode: 'rainbow' },
  { key: '2', name: 'Shifting', colorMode: 'cycle' },
  { key: '3', name: 'Black', colorMode: 'solid', color: '#000000' },
  { key: '4', name: 'White', colorMode: 'solid', color: '#ffffff' },
  { key: '5', name: 'Green', colorMode: 'solid', color: CYCLE_COLORS[2] },
  { key: '6', name: 'Yellow', colorMode: 'solid', color: CYCLE_COLORS[1] },
  { key: '7', name: 'Red', colorMode: 'solid', color: CYCLE_COLORS[0] },
  { key: '8', name: 'Blue', colorMode: 'solid', color: CYCLE_COLORS[4] },
];

export function colorShortcut(event: KeyEvent): { colorMode: ColorMode; color?: string } | undefined {
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
  const choice = COLOR_SHORTCUTS.find(choice => choice.key === event.key);
  if (!choice) return;
  return choice.color ? { colorMode: choice.colorMode, color: choice.color } : { colorMode: choice.colorMode };
}

export function toolShortcut(event: KeyEvent): Tool | undefined {
  if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) return;
  return TOOL_SHORTCUTS.find(tool => tool.key.toLowerCase() === event.key.toLowerCase())?.id;
}

// Key codes the native shortcut parser accepts verbatim. Escape cancels recording instead.
const RECORDABLE = /^(Key[A-Z]|Digit\d|F([1-9]|1\d|2[0-4])|Numpad(\d|Add|Subtract|Multiply|Divide|Decimal|Enter|Equal)|Arrow(Up|Down|Left|Right)|Space|Enter|Tab|Backspace|Delete|Insert|Home|End|Page(Up|Down)|Backquote|Backslash|Bracket(Left|Right)|Comma|Period|Slash|Semicolon|Quote|Minus|Equal)$/;
export const MODIFIER_KEYS = new Set(['Meta', 'Control', 'Alt', 'Shift', 'CapsLock', 'Fn', 'AltGraph', 'Hyper', 'Super', 'OS']);

/** Modifiers held in an event, in the order the shortcut string lists them. */
export function heldModifiers(event: Omit<KeyEvent, 'key'>, mac: boolean): string[] {
  const command = mac ? event.metaKey : event.ctrlKey;
  const secondary = mac ? event.ctrlKey : event.metaKey;
  return [command && 'CommandOrControl', secondary && (mac ? 'Control' : 'Super'), event.altKey && 'Alt', event.shiftKey && 'Shift'].filter((m): m is string => Boolean(m));
}

/** Build a shortcut string from a key press, or undefined for modifier-only or unsupported keys. */
export function recordShortcut(event: KeyEvent & Pick<KeyboardEvent, 'code'>, mac: boolean): string | undefined {
  if (MODIFIER_KEYS.has(event.key) || !RECORDABLE.test(event.code)) return;
  return [...heldModifiers(event, mac), event.code].join('+');
}

const KEY_NAMES: Record<string, string> = {
  Space: 'Space', Enter: 'Enter', Tab: 'Tab', Backspace: 'Backspace', Delete: 'Delete', Insert: 'Insert', Home: 'Home', End: 'End',
  PageUp: 'Page Up', PageDown: 'Page Down', Backquote: '`', Backslash: '\\', BracketLeft: '[', BracketRight: ']', Comma: ',',
  Period: '.', Slash: '/', Semicolon: ';', Quote: "'", Minus: '-', Equal: '=', ArrowUp: '↑', ArrowDown: '↓', ArrowLeft: '←', ArrowRight: '→',
};
function keyName(code: string): string {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^Numpad/.test(code)) return `Num ${code.slice(6)}`;
  return KEY_NAMES[code] ?? code.toUpperCase();
}

/** Render a shortcut string using platform conventions. */
export function formatShortcut(shortcut: string, mac: boolean): string {
  const parts = shortcut.split('+').map(part => part.trim()).filter(Boolean).map(part => {
    const lower = part.toLowerCase();
    if (['commandorcontrol', 'cmdorctrl', 'commandorctrl', 'cmdorcontrol'].includes(lower)) return mac ? '⌘' : 'Ctrl';
    if (['command', 'cmd', 'super'].includes(lower)) return mac ? '⌘' : 'Win';
    if (['control', 'ctrl'].includes(lower)) return mac ? '⌃' : 'Ctrl';
    if (['alt', 'option'].includes(lower)) return mac ? '⌥' : 'Alt';
    if (lower === 'shift') return mac ? '⇧' : 'Shift';
    return keyName(part);
  });
  return parts.join(mac ? '' : '+');
}
