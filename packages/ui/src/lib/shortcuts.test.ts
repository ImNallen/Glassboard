import { describe, expect, it } from 'vitest';
import { colorShortcut, commandFor, formatShortcut, heldModifiers, KEYBINDINGS, keybinding, matchesShortcut, normalizeShortcut, recordShortcut, sameShortcut, shortcutKeys, toolShortcut } from './shortcuts';

describe('color shortcuts', () => {
  const event = { key: '1', metaKey: false, ctrlKey: false, altKey: false, shiftKey: false };
  it('maps bare 1–8 to toolbar order', () => {
    const choices = [
      { colorMode: 'rainbow' }, { colorMode: 'cycle' },
      { colorMode: 'solid', color: '#000000' }, { colorMode: 'solid', color: '#ffffff' },
      { colorMode: 'solid', color: '#4dcaa0' }, { colorMode: 'solid', color: '#f2c85b' },
      { colorMode: 'solid', color: '#f46b78' }, { colorMode: 'solid', color: '#669df0' },
    ];
    choices.forEach((choice, index) => expect(colorShortcut({ ...event, key: String(index + 1) })).toEqual(choice));
  });
  it('leaves modified numbers and unrelated keys alone', () => {
    for (const modifier of ['metaKey', 'ctrlKey', 'altKey', 'shiftKey']) {
      expect(colorShortcut({ ...event, [modifier]: true })).toBeUndefined();
    }
    for (const key of ['x', 'X', '0', '9', 'ArrowUp']) {
      expect(colorShortcut({ ...event, key })).toBeUndefined();
    }
  });
});

describe('tool shortcuts', () => {
  const event = { key: '1', metaKey: false, ctrlKey: false, altKey: false, shiftKey: false };
  it.each(['metaKey', 'ctrlKey'] as const)('selects tools with %s and their assigned keys', modifier => {
    for (const [key, tool] of Object.entries({ '1': 'arrow', '2': 'pen', '3': 'rectangle', '4': 'ellipse', e: 'eraser', t: 'text', h: 'highlighter', E: 'eraser', T: 'text', H: 'highlighter' })) {
      expect(toolShortcut({ ...event, key, [modifier]: true })).toBe(tool);
    }
  });
  it('ignores bare numbers, extra modifiers, removed bindings, and other numbers', () => {
    for (const key of ['1', '2', '3', '4', 'e', 't', 'h']) expect(toolShortcut({ ...event, key })).toBeUndefined();
    expect(toolShortcut({ ...event, metaKey: true, shiftKey: true })).toBeUndefined();
    expect(toolShortcut({ ...event, ctrlKey: true, altKey: true })).toBeUndefined();
    for (const key of ['a', 'r', 'o', 'p', 'x', 'X', '5', '6', '7', '8', '0']) {
      expect(toolShortcut({ ...event, key, metaKey: true })).toBeUndefined();
    }
  });
});

describe('shortcut recording', () => {
  const press = (code: string, key: string, mods: Partial<Record<'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey', boolean>> = {}) =>
    ({ code, key, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...mods });
  it('maps the platform command key to CommandOrControl', () => {
    expect(recordShortcut(press('KeyA', 'a', { metaKey: true, shiftKey: true }), true)).toBe('CommandOrControl+Shift+KeyA');
    expect(recordShortcut(press('KeyA', 'a', { ctrlKey: true, shiftKey: true }), false)).toBe('CommandOrControl+Shift+KeyA');
    expect(recordShortcut(press('KeyA', 'a', { ctrlKey: true }), true)).toBe('Control+KeyA');
    expect(recordShortcut(press('KeyA', 'a', { metaKey: true }), false)).toBe('Super+KeyA');
  });
  it('accepts keys the native parser understands and ignores modifier-only presses', () => {
    expect(recordShortcut(press('F5', 'F5', { altKey: true }), true)).toBe('Alt+F5');
    expect(recordShortcut(press('Digit1', '1', { altKey: true, shiftKey: true }), false)).toBe('Alt+Shift+Digit1');
    expect(recordShortcut(press('Space', ' ', { ctrlKey: true }), true)).toBe('Control+Space');
    expect(recordShortcut(press('Shift', 'Shift', { shiftKey: true }), true)).toBeUndefined();
    expect(recordShortcut(press('MetaLeft', 'Meta', { metaKey: true }), true)).toBeUndefined();
    expect(recordShortcut(press('IntlBackslash', '<', { metaKey: true }), true)).toBeUndefined();
    expect(recordShortcut(press('Escape', 'Escape', { metaKey: true }), true)).toBeUndefined();
  });
  it('records an unmodified key so native validation can explain the rejection', () => {
    expect(recordShortcut(press('KeyB', 'b'), true)).toBe('KeyB');
    expect(heldModifiers(press('KeyB', 'b'), true)).toEqual([]);
  });
});

describe('shortcut labels', () => {
  it('renders macOS symbols without separators', () => {
    expect(formatShortcut('CommandOrControl+Shift+A', true)).toBe('⌘⇧A');
    expect(formatShortcut('CommandOrControl+Shift+KeyA', true)).toBe('⌘⇧A');
    expect(formatShortcut('Control+Alt+Digit1', true)).toBe('⌃⌥1');
    expect(formatShortcut('Alt+ArrowUp', true)).toBe('⌥↑');
  });
  it('renders Windows names with plus signs', () => {
    expect(formatShortcut('CommandOrControl+Shift+KeyA', false)).toBe('Ctrl+Shift+A');
    expect(formatShortcut('Super+Comma', false)).toBe('Win+,');
    expect(formatShortcut('Alt+NumpadAdd', false)).toBe('Alt+Num Add');
    expect(formatShortcut('Control+F12', false)).toBe('Ctrl+F12');
  });
});

describe('keybindings', () => {
  const press = (key: string, code: string, mods: Partial<Record<'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey', boolean>> = {}) =>
    ({ key, code, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...mods });
  it('gives every command a unique default', () => {
    const ids = KEYBINDINGS.map(binding => binding.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const mac of [true, false]) {
      const shortcuts = KEYBINDINGS.map(binding => normalizeShortcut(binding.shortcut, mac));
      expect(new Set(shortcuts).size).toBe(shortcuts.length);
    }
    for (const id of ids) expect(id).toMatch(/^[a-z0-9-]{1,32}$/);
  });
  it('uses saved bindings over defaults, and an empty binding unbinds', () => {
    expect(keybinding(undefined, 'undo')).toBe('CommandOrControl+KeyZ');
    expect(keybinding({ undo: 'Alt+KeyU' }, 'undo')).toBe('Alt+KeyU');
    expect(keybinding({ undo: '' }, 'undo')).toBe('');
    expect(commandFor(press('z', 'KeyZ', { metaKey: true }), { undo: 'Alt+KeyU' }, true)).toBeUndefined();
    expect(commandFor(press('u', 'KeyU', { altKey: true }), { undo: 'Alt+KeyU' }, true)).toBe('undo');
    expect(commandFor(press('1', 'Digit1'), { 'color-rainbow': '' }, true)).toBeUndefined();
    expect(toolShortcut(press('p', 'KeyP'), { 'tool-pen': 'KeyP' }, true)).toBe('pen');
    expect(colorShortcut(press('r', 'KeyR', { shiftKey: true }), { 'color-red': 'Shift+KeyR' }, true)).toEqual({ colorMode: 'solid', color: '#f46b78' });
  });
  it('matches letters by layout and other keys by position', () => {
    // AZERTY: the physical Q key types "a".
    expect(matchesShortcut(press('a', 'KeyQ', { metaKey: true }), 'CommandOrControl+KeyA', true)).toBe(true);
    expect(matchesShortcut(press('!', 'Digit1', { shiftKey: true }), 'Shift+Digit1', true)).toBe(true);
    expect(matchesShortcut(press('å', 'KeyA', { altKey: true }), 'Alt+KeyA', true)).toBe(true);
    expect(matchesShortcut(press('Escape', 'Escape'), 'Escape', true)).toBe(true);
    expect(matchesShortcut(press('Escape', 'Escape', { shiftKey: true }), 'Escape', true)).toBe(false);
    expect(matchesShortcut(press('[', 'BracketLeft', { ctrlKey: true }), 'CommandOrControl+BracketLeft', false)).toBe(true);
  });
  it('separates the second platform modifier when a binding names it', () => {
    expect(matchesShortcut(press('k', 'KeyK', { ctrlKey: true }), 'Control+KeyK', true)).toBe(true);
    expect(matchesShortcut(press('k', 'KeyK', { metaKey: true }), 'Control+KeyK', true)).toBe(false);
    expect(matchesShortcut(press('k', 'KeyK', { metaKey: true, ctrlKey: true }), 'CommandOrControl+Control+KeyK', true)).toBe(true);
    expect(matchesShortcut(press('k', 'KeyK', { metaKey: true }), 'CommandOrControl+Control+KeyK', true)).toBe(false);
  });
  it('keeps a second-modifier binding reachable next to a CommandOrControl binding on the same key', () => {
    const mac = { undo: 'CommandOrControl+KeyK', redo: 'Control+KeyK' };
    expect(commandFor(press('k', 'KeyK', { metaKey: true }), mac, true)).toBe('undo');
    expect(commandFor(press('k', 'KeyK', { ctrlKey: true }), mac, true)).toBe('redo');
    const windows = { undo: 'CommandOrControl+KeyK', redo: 'Super+KeyK' };
    expect(commandFor(press('k', 'KeyK', { ctrlKey: true }), windows, false)).toBe('undo');
    expect(commandFor(press('k', 'KeyK', { metaKey: true }), windows, false)).toBe('redo');
    expect(toolShortcut(press('k', 'KeyK', { ctrlKey: true }), { 'tool-pen': 'CommandOrControl+KeyK', 'tool-text': 'Control+KeyK' }, true)).toBe('text');
    // With nothing else on the key, either command key still works for a CommandOrControl binding.
    expect(commandFor(press('z', 'KeyZ', { ctrlKey: true }), {}, true)).toBe('undo');
    expect(commandFor(press('z', 'KeyZ', { metaKey: true }), {}, false)).toBe('undo');
  });
  it('compares shortcuts written in different forms', () => {
    expect(sameShortcut('CommandOrControl+Shift+A', 'Shift+CmdOrCtrl+KeyA')).toBe(true);
    expect(sameShortcut('Super+KeyA', 'CommandOrControl+KeyA', true)).toBe(true);
    expect(sameShortcut('Control+KeyA', 'CommandOrControl+KeyA', false)).toBe(true);
    expect(sameShortcut('Control+KeyA', 'CommandOrControl+KeyA', true)).toBe(false);
    expect(sameShortcut('', '')).toBe(true);
    expect(sameShortcut('', 'KeyA')).toBe(false);
  });
  it('records layout letters for in-app bindings and positions for the global shortcut', () => {
    const azertyA = press('a', 'KeyQ', { metaKey: true });
    expect(recordShortcut(azertyA, true)).toBe('CommandOrControl+KeyA');
    expect(recordShortcut(azertyA, true, true)).toBe('CommandOrControl+KeyQ');
    expect(recordShortcut(press('3', 'Digit3'), true)).toBe('Digit3');
  });
  it('splits shortcuts into keycap labels', () => {
    expect(shortcutKeys('CommandOrControl+Shift+KeyZ', true)).toEqual(['⌘', '⇧', 'Z']);
    expect(shortcutKeys('CommandOrControl+Shift+KeyZ', false)).toEqual(['Ctrl', 'Shift', 'Z']);
    expect(shortcutKeys('Escape', true)).toEqual(['Esc']);
    expect(shortcutKeys('', true)).toEqual([]);
  });
});
