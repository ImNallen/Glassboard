import { describe, expect, it } from 'vitest';
import { eraseShortcut, toolShortcut } from './shortcuts';

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
  it.each(['metaKey', 'ctrlKey'] as const)('selects the four tools with %s and 1–4', modifier => {
    for (const [index, tool] of ['arrow', 'rectangle', 'ellipse', 'highlighter'].entries()) {
      expect(toolShortcut({ ...event, key: String(index + 1), [modifier]: true })).toBe(tool);
    }
  });
  it('ignores bare numbers, extra modifiers, removed bindings, and other numbers', () => {
    expect(toolShortcut(event)).toBeUndefined();
    expect(toolShortcut({ ...event, metaKey: true, shiftKey: true })).toBeUndefined();
    expect(toolShortcut({ ...event, ctrlKey: true, altKey: true })).toBeUndefined();
    for (const key of ['a', 'r', 'o', 'p', 'h', '5']) {
      expect(toolShortcut({ ...event, key, metaKey: true })).toBeUndefined();
    }
  });
});
