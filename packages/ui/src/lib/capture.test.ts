// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { captureKeydown, capturePixels, captureRegion } from './capture';

describe('capture geometry', () => {
  it('supports reverse drags, negative coordinates, and the display edge', () => {
    expect(captureRegion({ x: 300, y: 180 }, { x: -20, y: 20 }, 500, 300)).toEqual({ x: 0, y: 20, width: 300, height: 160 });
    expect(captureRegion({ x: 100, y: 200 }, { x: 900, y: 600 }, 500, 300)).toEqual({ x: 100, y: 200, width: 400, height: 100 });
  });
  it('uses screenshot density and includes fractional edge pixels without exceeding the image', () => {
    expect(capturePixels({ x: 10.25, y: 20.75, width: 40.5, height: 30.5 }, { width: 500, height: 300 }, { width: 1000, height: 600 })).toEqual({ x: 20, y: 41, width: 82, height: 62 });
    expect(capturePixels({ x: 490, y: 290, width: 100, height: 100 }, { width: 500, height: 300 }, { width: 750, height: 450 })).toEqual({ x: 735, y: 435, width: 15, height: 15 });
  });
});

function key(key: string, modifiers: KeyboardEventInit, target: HTMLElement = document.body) {
  const event = new KeyboardEvent('keydown', { key, cancelable: true, ...modifiers });
  Object.defineProperty(event, 'target', { value: target });
  return event;
}

describe('capture shortcuts', () => {
  it.each([{ metaKey: true }, { ctrlKey: true }])('copies only with C using %j', modifier => {
    const copy = vi.fn(), cancel = vi.fn();
    const event = key('c', modifier);
    captureKeydown(event, { copy, cancel });
    expect(event.defaultPrevented).toBe(true);
    const save = key('s', modifier);
    captureKeydown(save, { copy, cancel });
    expect(save.defaultPrevented).toBe(false);
    expect(copy).toHaveBeenCalledOnce();
    expect(cancel).not.toHaveBeenCalled();
  });
  it('leaves text copy, Save, and Escape to the text editor', () => {
    const copy = vi.fn(), cancel = vi.fn(), textarea = document.createElement('textarea');
    const textCopy = key('c', { metaKey: true }, textarea), escape = key('Escape', {}, textarea);
    captureKeydown(textCopy, { copy, cancel });
    captureKeydown(escape, { copy, cancel });
    expect(textCopy.defaultPrevented).toBe(false);
    expect(escape.defaultPrevented).toBe(false);
    expect(copy).not.toHaveBeenCalled();
    captureKeydown(key('s', { metaKey: true }, textarea), { copy, cancel });
    expect(copy).not.toHaveBeenCalled();
  });
  it('cancels from the canvas and leaves capture/global/clipboard modifiers alone', () => {
    const copy = vi.fn(), cancel = vi.fn();
    captureKeydown(key('Escape', {}), { copy, cancel });
    captureKeydown(key('Escape', { repeat: true }), { copy, cancel });
    expect(cancel).toHaveBeenCalledOnce();
    for (const modifiers of [{ metaKey: true, shiftKey: true }, { ctrlKey: true, altKey: true }, { metaKey: true, repeat: true }]) {
      captureKeydown(key('c', modifiers), { copy, cancel });
    }
    expect(copy).not.toHaveBeenCalled();
  });
  it('follows rebound copy and cancel keys', () => {
    const copy = vi.fn(), cancel = vi.fn(), keybindings = { copy: 'CommandOrControl+Shift+KeyC', hide: 'KeyQ' };
    captureKeydown(key('c', { metaKey: true }), { copy, cancel, keybindings });
    captureKeydown(key('Escape', {}), { copy, cancel, keybindings });
    expect(copy).not.toHaveBeenCalled();
    expect(cancel).not.toHaveBeenCalled();
    captureKeydown(key('c', { metaKey: true, shiftKey: true }), { copy, cancel, keybindings });
    captureKeydown(key('q', {}), { copy, cancel, keybindings });
    expect(copy).toHaveBeenCalledOnce();
    expect(cancel).toHaveBeenCalledOnce();
  });
});
