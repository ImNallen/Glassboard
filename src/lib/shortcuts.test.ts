import { describe, expect, it } from 'vitest';
import { colorShortcut, formatShortcut, heldModifiers, recordShortcut, toolShortcut } from './shortcuts';

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
