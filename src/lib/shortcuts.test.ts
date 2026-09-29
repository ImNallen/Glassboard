import { describe, expect, it } from 'vitest';
import { eraseShortcut, formatShortcut, heldModifiers, recordShortcut, toolShortcut } from './shortcuts';

describe('erase shortcut', () => {
  const event = { key: 'x', metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, repeat: false };
  it('accepts X, including Caps Lock', () => {
    expect(eraseShortcut(event)).toBe(true);
    expect(eraseShortcut({ ...event, key: 'X' })).toBe(true);
  });
  it('ignores modifiers, key repeats, and unrelated keys', () => {
    for (const modifier of ['metaKey', 'ctrlKey', 'altKey', 'shiftKey', 'repeat']) {
      expect(eraseShortcut({ ...event, [modifier]: true })).toBe(false);
    }
    expect(eraseShortcut({ ...event, key: 'Delete' })).toBe(false);
  });
});

describe('tool shortcuts', () => {
  const event = { key: '1', metaKey: false, ctrlKey: false, altKey: false, shiftKey: false };
  it.each(['metaKey', 'ctrlKey'] as const)('selects the seven tools with %s and 1–7', modifier => {
    for (const [index, tool] of ['pen', 'arrow', 'rectangle', 'ellipse', 'highlighter', 'text', 'eraser'].entries()) {
      expect(toolShortcut({ ...event, key: String(index + 1), [modifier]: true })).toBe(tool);
    }
  });
  it('ignores bare numbers, extra modifiers, removed bindings, and other numbers', () => {
    expect(toolShortcut(event)).toBeUndefined();
    expect(toolShortcut({ ...event, metaKey: true, shiftKey: true })).toBeUndefined();
    expect(toolShortcut({ ...event, ctrlKey: true, altKey: true })).toBeUndefined();
    for (const key of ['a', 'r', 'o', 'p', 'h', '8', '0']) {
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
