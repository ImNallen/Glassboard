import preferenceDefaults from '../contract/preference-defaults.json';
import type { ColorMode, Tool } from './drawing';
import { swatchColor } from './swatches';

export const DEFAULT_SHORTCUT = preferenceDefaults.shortcut;
export const DEFAULT_CAPTURE_SHORTCUT = preferenceDefaults.captureShortcut;
type KeyEvent = Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'> & Partial<Pick<KeyboardEvent, 'code'>>;
export const platformMac = typeof navigator !== 'undefined' && navigator.platform.toLowerCase().includes('mac');

/** Saved changes to in-app bindings, keyed by command id. An empty string leaves the command unbound. */
export type Keybindings = Partial<Record<string, string>>;
export type KeybindingGroup = 'Drawing' | 'Tools' | 'Colors' | 'Screenshot';
export const KEYBINDING_GROUPS: readonly KeybindingGroup[] = ['Drawing', 'Tools', 'Colors', 'Screenshot'];

export const TOOL_SHORTCUTS: readonly { id: Tool; command: string; name: string; shortcut: string }[] = [
  { id: 'arrow', command: 'tool-arrow', name: 'Arrow', shortcut: 'CommandOrControl+Digit1' },
  { id: 'pen', command: 'tool-pen', name: 'Pen', shortcut: 'CommandOrControl+Digit2' },
  { id: 'rectangle', command: 'tool-rectangle', name: 'Square', shortcut: 'CommandOrControl+Digit3' },
  { id: 'ellipse', command: 'tool-ellipse', name: 'Circle', shortcut: 'CommandOrControl+Digit4' },
  { id: 'eraser', command: 'tool-eraser', name: 'Eraser', shortcut: 'CommandOrControl+KeyE' },
  { id: 'text', command: 'tool-text', name: 'Text', shortcut: 'CommandOrControl+KeyT' },
  { id: 'highlighter', command: 'tool-highlighter', name: 'Highlighter', shortcut: 'CommandOrControl+KeyH' },
];

/** Solid colors select a swatch slot, whose color the user can change. Command ids keep their default color names. */
export const COLOR_SHORTCUTS: readonly { command: string; name: string; colorMode: ColorMode; slot?: number; shortcut: string }[] = [
  { command: 'color-rainbow', name: 'Rainbow', colorMode: 'rainbow', shortcut: 'Digit1' },
  { command: 'color-cycle', name: 'Shifting', colorMode: 'cycle', shortcut: 'Digit2' },
  { command: 'color-black', name: 'Black', colorMode: 'solid', slot: 0, shortcut: 'Digit3' },
  { command: 'color-white', name: 'White', colorMode: 'solid', slot: 1, shortcut: 'Digit4' },
  { command: 'color-green', name: 'Green', colorMode: 'solid', slot: 2, shortcut: 'Digit5' },
  { command: 'color-yellow', name: 'Yellow', colorMode: 'solid', slot: 3, shortcut: 'Digit6' },
  { command: 'color-red', name: 'Red', colorMode: 'solid', slot: 4, shortcut: 'Digit7' },
  { command: 'color-blue', name: 'Blue', colorMode: 'solid', slot: 5, shortcut: 'Digit8' },
];

/** Every in-app binding with its default. Global shortcuts live in `Preferences.shortcut` and `Preferences.captureShortcut`. */
export const KEYBINDINGS: readonly { id: string; group: KeybindingGroup; name: string; shortcut: string; description?: string }[] = [
  { id: 'undo', group: 'Drawing', name: 'Undo', shortcut: 'CommandOrControl+KeyZ' },
  { id: 'redo', group: 'Drawing', name: 'Redo', shortcut: 'CommandOrControl+Shift+KeyZ' },
  { id: 'hide', group: 'Drawing', name: 'Close Glassboard', shortcut: 'Escape' },
  ...TOOL_SHORTCUTS.map(tool => ({ id: tool.command, group: 'Tools' as const, name: tool.name, shortcut: tool.shortcut })),
  ...COLOR_SHORTCUTS.map(color => ({ id: color.command, group: 'Colors' as const, name: color.name, shortcut: color.shortcut })),
  { id: 'capture', group: 'Screenshot', name: 'Take screenshot', shortcut: 'CommandOrControl+KeyS' },
  { id: 'copy', group: 'Screenshot', name: 'Copy & close', shortcut: 'CommandOrControl+KeyC', description: 'In the screenshot editor' },
];

/** The shortcut bound to a command: the saved choice, or the default. */
export function keybinding(keybindings: Keybindings | undefined, id: string): string {
  return keybindings?.[id] ?? KEYBINDINGS.find(binding => binding.id === id)?.shortcut ?? '';
}

/** The label of the shortcut bound to a command, empty when unbound. */
export function keyLabel(keybindings: Keybindings | undefined, id: string): string { return shortcutLabel(keybinding(keybindings, id)); }

/** Commands on system-wide shortcuts, which work even while Glassboard is hidden. */
export type GlobalCommand = 'toggle' | 'capture';
const GLOBAL_FIELDS = { toggle: 'shortcut', capture: 'captureShortcut' } as const;
/** The global shortcuts and the in-app bindings, which must not share a shortcut. An empty `captureShortcut` is unbound. */
export type Bindings = { shortcut: string; captureShortcut: string; keybindings: Keybindings };
/** New bindings, with the names of the commands unbound to make room. */
export type Rebound = Bindings & { moved: string[] };

function unbind(keybindings: Keybindings, shortcut: string, mac: boolean, except?: string): Pick<Rebound, 'keybindings' | 'moved'> {
  const taken = KEYBINDINGS.filter(binding => binding.id !== except && sameShortcut(keybinding(keybindings, binding.id), shortcut, mac));
  return { keybindings: { ...keybindings, ...Object.fromEntries(taken.map(binding => [binding.id, ''])) }, moved: taken.map(binding => binding.name) };
}

function globalOwner(current: Bindings, shortcut: string, mac: boolean, except?: GlobalCommand): GlobalCommand | undefined {
  return (['toggle', 'capture'] as const).find(command => command !== except && current[GLOBAL_FIELDS[command]] !== '' && sameShortcut(current[GLOBAL_FIELDS[command]], shortcut, mac));
}

/**
 * Bind `next` to a global command, unbinding any in-app command that uses it; an empty `next` unbinds capture.
 * Undefined when nothing changes, the other global command when it owns `next`.
 */
export function rebindGlobal(current: Bindings, command: GlobalCommand, next: string, mac = platformMac): Rebound | GlobalCommand | undefined {
  const field = GLOBAL_FIELDS[command];
  if (sameShortcut(next, current[field], mac) || (!next && command === 'toggle')) return;
  const owner = next ? globalOwner(current, next, mac, command) : undefined;
  if (owner) return owner;
  const rebound: Rebound = { shortcut: current.shortcut, captureShortcut: current.captureShortcut, ...(next ? unbind(current.keybindings, next, mac) : { keybindings: { ...current.keybindings }, moved: [] }) };
  rebound[field] = next;
  return rebound;
}

/**
 * Bind `next` to a command, unbinding any other command that uses it; an empty `next` unbinds the command.
 * A binding equal to the default is removed from the map. Undefined when nothing changes, the global command that owns `next` when one does.
 */
export function rebindCommand(current: Bindings, id: string, next: string, mac = platformMac): Rebound | GlobalCommand | undefined {
  if (sameShortcut(next, keybinding(current.keybindings, id), mac)) return;
  const owner = next ? globalOwner(current, next, mac) : undefined;
  if (owner) return owner;
  const { keybindings, moved } = next ? unbind(current.keybindings, next, mac, id) : { keybindings: { ...current.keybindings }, moved: [] };
  if (sameShortcut(next, keybinding(undefined, id), mac)) delete keybindings[id];
  else keybindings[id] = next;
  return { shortcut: current.shortcut, captureShortcut: current.captureShortcut, keybindings, moved };
}

/**
 * The in-app command bound to a key press, if any. An exact match wins, so a binding on the
 * platform's second modifier (Control on macOS, Super on Windows) isn't shadowed by a
 * CommandOrControl binding on the same key, which otherwise accepts either modifier.
 */
export function commandFor(event: KeyEvent, keybindings: Keybindings | undefined, mac = platformMac): string | undefined {
  const bound = (exact: boolean) => KEYBINDINGS.find(binding => matchesShortcut(event, keybinding(keybindings, binding.id), mac, exact))?.id;
  return bound(true) ?? bound(false);
}

/** The preference change a color shortcut makes. */
export function colorShortcut(event: KeyEvent, keybindings?: Keybindings, mac = platformMac, swatches?: readonly string[]): { colorMode: ColorMode; color?: string } | undefined {
  const command = commandFor(event, keybindings, mac);
  const choice = COLOR_SHORTCUTS.find(choice => choice.command === command);
  if (!choice) return;
  return choice.slot === undefined ? { colorMode: choice.colorMode } : { colorMode: choice.colorMode, color: swatchColor(swatches, choice.slot) };
}

export function toolShortcut(event: KeyEvent, keybindings?: Keybindings, mac = platformMac): Tool | undefined {
  const command = commandFor(event, keybindings, mac);
  return TOOL_SHORTCUTS.find(tool => tool.command === command)?.id;
}

const MODIFIER_ORDER = ['CommandOrControl', 'Control', 'Super', 'Alt', 'Shift'];
const COMMAND_ALIASES = ['commandorcontrol', 'cmdorctrl', 'commandorctrl', 'cmdorcontrol'];

/** Canonical form for comparison: platform command key as CommandOrControl, modifiers in recording order, letters and digits as key codes. */
export function normalizeShortcut(shortcut: string, mac = platformMac): string {
  const modifiers = new Set<string>();
  let key = '';
  for (const part of shortcut.split('+').map(part => part.trim()).filter(Boolean)) {
    const lower = part.toLowerCase();
    if (COMMAND_ALIASES.includes(lower)) modifiers.add('CommandOrControl');
    else if (['command', 'cmd', 'super'].includes(lower)) modifiers.add(mac ? 'CommandOrControl' : 'Super');
    else if (['control', 'ctrl'].includes(lower)) modifiers.add(mac ? 'Control' : 'CommandOrControl');
    else if (['alt', 'option'].includes(lower)) modifiers.add('Alt');
    else if (lower === 'shift') modifiers.add('Shift');
    else key = /^[a-z]$/i.test(part) ? `Key${part.toUpperCase()}` : /^\d$/.test(part) ? `Digit${part}` : part;
  }
  return [...MODIFIER_ORDER.filter(modifier => modifiers.has(modifier)), key].filter(Boolean).join('+');
}

export function sameShortcut(a: string, b: string, mac = platformMac): boolean {
  return normalizeShortcut(a, mac) === normalizeShortcut(b, mac);
}

// Letters and digits follow the keyboard layout; other keys use their physical position.
function eventKey(event: KeyEvent, physical = false): string {
  if (!physical && /^[a-z]$/i.test(event.key)) return `Key${event.key.toUpperCase()}`;
  if (!physical && /^\d$/.test(event.key)) return `Digit${event.key}`;
  return event.code || event.key;
}

/** `exact` requires the platform command modifier itself for CommandOrControl, instead of either command key. */
export function matchesShortcut(event: KeyEvent, shortcut: string, mac = platformMac, exact = false): boolean {
  if (!shortcut) return false;
  const modifiers = normalizeShortcut(shortcut, mac).split('+');
  if (modifiers.pop() !== eventKey(event)) return false;
  const command = mac ? event.metaKey : event.ctrlKey, secondary = mac ? event.ctrlKey : event.metaKey;
  const wantsCommand = modifiers.includes('CommandOrControl'), wantsSecondary = modifiers.includes(mac ? 'Control' : 'Super');
  // Either command key works for a plain CommandOrControl binding, as Cmd and Ctrl did before rebinding.
  const commandMatches = wantsCommand && !wantsSecondary && !exact ? command || secondary : command === wantsCommand && secondary === wantsSecondary;
  return commandMatches && event.altKey === modifiers.includes('Alt') && event.shiftKey === modifiers.includes('Shift');
}

// Key codes the native shortcut parser accepts verbatim (contract/toggle-shortcuts.json checks both sides agree). Escape cancels recording instead; resetting restores an Escape default.
const RECORDABLE = /^(Key[A-Z]|Digit\d|F([1-9]|1\d|2[0-4])|Numpad(\d|Add|Subtract|Multiply|Divide|Decimal|Enter|Equal)|Arrow(Up|Down|Left|Right)|Space|Enter|Tab|Backspace|Delete|Insert|Home|End|Page(Up|Down)|Backquote|Backslash|Bracket(Left|Right)|Comma|Period|Slash|Semicolon|Quote|Minus|Equal)$/;
export const MODIFIER_KEYS = new Set(['Meta', 'Control', 'Alt', 'Shift', 'CapsLock', 'Fn', 'AltGraph', 'Hyper', 'Super', 'OS']);

/** Modifiers held in an event, in the order the shortcut string lists them. */
export function heldModifiers(event: Omit<KeyEvent, 'key'>, mac: boolean): string[] {
  const command = mac ? event.metaKey : event.ctrlKey;
  const secondary = mac ? event.ctrlKey : event.metaKey;
  return [command && 'CommandOrControl', secondary && (mac ? 'Control' : 'Super'), event.altKey && 'Alt', event.shiftKey && 'Shift'].filter((m): m is string => Boolean(m));
}

/**
 * Build a shortcut string from a key press, or undefined for modifier-only or unsupported keys.
 * `physical` records key positions, as the native global shortcut needs; otherwise letters and digits follow the layout.
 */
export function recordShortcut(event: KeyEvent & Pick<KeyboardEvent, 'code'>, mac: boolean, physical = false): string | undefined {
  const key = eventKey(event, physical);
  if (MODIFIER_KEYS.has(event.key) || !RECORDABLE.test(key)) return;
  return [...heldModifiers(event, mac), key].join('+');
}

const KEY_NAMES: Record<string, string> = {
  Escape: 'Esc', Space: 'Space', Enter: 'Enter', Tab: 'Tab', Backspace: 'Backspace', Delete: 'Delete', Insert: 'Insert', Home: 'Home', End: 'End',
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
  return shortcutKeys(shortcut, mac).join(mac ? '' : '+');
}
export function shortcutLabel(shortcut: string) { return formatShortcut(shortcut, platformMac); }

/** The labels of each key in a shortcut, for rendering as separate keycaps. */
export function shortcutKeys(shortcut: string, mac: boolean): string[] {
  return shortcut.split('+').map(part => part.trim()).filter(Boolean).map(part => {
    const lower = part.toLowerCase();
    if (COMMAND_ALIASES.includes(lower)) return mac ? '⌘' : 'Ctrl';
    if (['command', 'cmd', 'super'].includes(lower)) return mac ? '⌘' : 'Win';
    if (['control', 'ctrl'].includes(lower)) return mac ? '⌃' : 'Ctrl';
    if (['alt', 'option'].includes(lower)) return mac ? '⌥' : 'Alt';
    if (lower === 'shift') return mac ? '⇧' : 'Shift';
    return keyName(part);
  });
}
