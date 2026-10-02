import { describe, expect, it } from 'vitest';
import grammar from './toggle-shortcuts.json';
import { normalizeShortcut, recordShortcut } from '../lib/shortcuts';

const press = (code: string) => ({ code, key: 'x', metaKey: false, ctrlKey: true, altKey: false, shiftKey: false });

// preferences.rs parses the same file with the native shortcut parser.
describe('the shortcut recorder follows the shared toggle grammar', () => {
  it('records every key the native parser accepts', () => {
    for (const code of grammar.recordableKeys) expect(recordShortcut(press(code), false, true), code).toBe(`CommandOrControl+${code}`);
  });
  it('normalizes every valid toggle shortcut to a key it can record', () => {
    for (const shortcut of grammar.toggle.valid) {
      const key = normalizeShortcut(shortcut, true).split('+').pop();
      expect(grammar.recordableKeys, shortcut).toContain(key);
    }
  });
});
