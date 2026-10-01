// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { drawingKeydown } from './keys';
import { defaults, type Session } from './session';

function press(key: string, init: KeyboardEventInit = {}, target: EventTarget = document.body) {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
  Object.defineProperty(event, 'target', { value: target });
  return event;
}
const session = (mode: Session['mode']): Session => ({ ...structuredClone(defaults), mode });

describe('drawingKeydown', () => {
  it('mirrors the show/hide shortcut in the browser by default', () => {
    const run = vi.fn();
    const event = press('a', { metaKey: true, shiftKey: true });
    drawingKeydown(event, session('hidden'), { run, save: vi.fn() });
    expect(run).toHaveBeenCalledWith('toggle');
    expect(event.defaultPrevented).toBe(true);
  });

  it('leaves the shortcut alone when the host opts out', () => {
    const run = vi.fn();
    const event = press('a', { metaKey: true, shiftKey: true });
    drawingKeydown(event, session('hidden'), { run, save: vi.fn(), toggleShortcut: false });
    expect(run).not.toHaveBeenCalled();
    expect(event.defaultPrevented).toBe(false);
  });

  it('still routes Escape, undo, tools, and colors while drawing with the shortcut off', () => {
    const run = vi.fn(), save = vi.fn();
    const handlers = { run, save, toggleShortcut: false };
    drawingKeydown(press('Escape'), session('draw'), handlers);
    expect(run).toHaveBeenCalledWith('hide');
    drawingKeydown(press('z', { metaKey: true, shiftKey: true }), session('draw'), handlers);
    expect(run).toHaveBeenCalledWith('redo');
    drawingKeydown(press('4', { metaKey: true }), session('draw'), handlers);
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ tool: 'ellipse' }));
    drawingKeydown(press('7'), session('draw'), handlers);
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ colorMode: 'solid', color: '#f46b78' }));
  });

  it('ignores drawing keys while hidden and while typing in a field', () => {
    const run = vi.fn(), save = vi.fn();
    drawingKeydown(press('Escape'), session('hidden'), { run, save });
    expect(run).not.toHaveBeenCalled();
    const input = document.createElement('input');
    drawingKeydown(press('z', { metaKey: true }, input), session('draw'), { run, save });
    expect(run).not.toHaveBeenCalled();
  });

  it.each(['metaKey', 'ctrlKey'] as const)('starts a capture with %s+S while drawing', modifier => {
    const capture = vi.fn(), run = vi.fn(), save = vi.fn();
    const event = press('s', { [modifier]: true });
    drawingKeydown(event, session('draw'), { run, save, capture });
    expect(event.defaultPrevented).toBe(true);
    expect(capture).toHaveBeenCalledOnce();
    expect(run).not.toHaveBeenCalled();
    expect(save).not.toHaveBeenCalled();
  });

  it('leaves Save alone outside capture-enabled drawing and ignores modified or repeated presses', () => {
    const capture = vi.fn(), handlers = { run: vi.fn(), save: vi.fn(), capture };
    const input = document.createElement('textarea');
    for (const event of [press('s'), press('s', { metaKey: true, shiftKey: true }), press('s', { ctrlKey: true, altKey: true }), press('s', { metaKey: true }, input)]) {
      drawingKeydown(event, session('draw'), handlers);
      expect(event.defaultPrevented).toBe(false);
    }
    const hidden = press('s', { metaKey: true });
    drawingKeydown(hidden, session('hidden'), handlers);
    expect(hidden.defaultPrevented).toBe(false);
    const unsupported = press('s', { metaKey: true });
    drawingKeydown(unsupported, session('draw'), { run: vi.fn(), save: vi.fn() });
    expect(unsupported.defaultPrevented).toBe(false);
    const repeat = press('s', { metaKey: true, repeat: true });
    drawingKeydown(repeat, session('draw'), handlers);
    expect(repeat.defaultPrevented).toBe(true);
    const handled = press('s', { metaKey: true });
    handled.preventDefault();
    drawingKeydown(handled, session('draw'), handlers);
    expect(capture).not.toHaveBeenCalled();
  });

  it('follows rebound and unbound commands', () => {
    const run = vi.fn(), save = vi.fn();
    const rebound = session('draw');
    rebound.preferences.keybindings = { undo: 'Alt+KeyU', hide: '', 'tool-pen': 'KeyP', 'color-blue': 'Shift+KeyB' };
    const handlers = { run, save, toggleShortcut: false };
    const oldUndo = press('z', { ctrlKey: true });
    drawingKeydown(oldUndo, rebound, handlers);
    expect(oldUndo.defaultPrevented).toBe(false);
    drawingKeydown(press('u', { altKey: true }), rebound, handlers);
    expect(run).toHaveBeenCalledWith('undo');
    const escape = press('Escape');
    drawingKeydown(escape, rebound, handlers);
    expect(escape.defaultPrevented).toBe(false);
    expect(run).not.toHaveBeenCalledWith('hide');
    drawingKeydown(press('p'), rebound, handlers);
    expect(save).toHaveBeenLastCalledWith(expect.objectContaining({ tool: 'pen' }));
    drawingKeydown(press('B', { shiftKey: true }), rebound, handlers);
    expect(save).toHaveBeenLastCalledWith(expect.objectContaining({ colorMode: 'solid', color: '#669df0' }));
  });

  it('mirrors a customized show/hide shortcut in the browser', () => {
    const run = vi.fn();
    const custom = session('hidden');
    custom.preferences.shortcut = 'Alt+Shift+KeyG';
    drawingKeydown(press('a', { ctrlKey: true, shiftKey: true }), custom, { run, save: vi.fn() });
    expect(run).not.toHaveBeenCalled();
    drawingKeydown(press('G', { altKey: true, shiftKey: true }), custom, { run, save: vi.fn() });
    expect(run).toHaveBeenCalledWith('toggle');
  });

  it('selects the color a user put in a swatch', () => {
    const save = vi.fn();
    const custom = session('draw');
    custom.preferences.swatches = ['#000000', '#ffffff', '#4dcaa0', '#f2c85b', '#8b5cf6', '#669df0'];
    drawingKeydown(press('7'), custom, { run: vi.fn(), save, toggleShortcut: false });
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ colorMode: 'solid', color: '#8b5cf6' }));
  });
});
