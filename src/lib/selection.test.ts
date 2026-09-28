// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { protectSelection } from './selection';

let stop: () => void;
let canvas: HTMLCanvasElement;
let input: HTMLInputElement;

function selectCanvas() {
  const range = document.createRange();
  range.selectNode(canvas);
  document.getSelection()!.removeAllRanges();
  document.getSelection()!.addRange(range);
}

beforeEach(() => {
  document.body.innerHTML = '<canvas></canvas><input value="CommandOrControl+Shift+A">';
  canvas = document.querySelector('canvas')!;
  input = document.querySelector('input')!;
  stop = protectSelection();
});

afterEach(() => {
  stop();
  document.getSelection()?.removeAllRanges();
  document.body.replaceChildren();
});

describe('overlay selection protection', () => {
  it.each(['metaKey', 'ctrlKey'])('blocks Select All with %s and clears a stuck canvas selection', modifier => {
    selectCanvas();
    const key = new KeyboardEvent('keydown', { key: 'a', [modifier]: true, bubbles: true, cancelable: true });
    canvas.dispatchEvent(key);
    expect(key.defaultPrevented).toBe(true);
    expect(document.getSelection()!.rangeCount).toBe(0);
  });

  it('clears selection made by the native menu without a keydown event', () => {
    selectCanvas();
    expect(document.getSelection()!.rangeCount).toBe(1);
    document.dispatchEvent(new Event('selectionchange'));
    expect(document.getSelection()!.rangeCount).toBe(0);
    // Follow-up selection notifications are harmless and leave the canvas intact.
    document.dispatchEvent(new Event('selectionchange'));
    expect(canvas.isConnected).toBe(true);
  });

  it('allows selecting and editing text in the shortcut field', () => {
    input.focus();
    const key = new KeyboardEvent('keydown', { key: 'a', metaKey: true, bubbles: true, cancelable: true });
    input.dispatchEvent(key);
    expect(key.defaultPrevented).toBe(false);
    const start = new Event('selectstart', { bubbles: true, cancelable: true });
    input.dispatchEvent(start);
    expect(start.defaultPrevented).toBe(false);
    input.select();
    document.dispatchEvent(new Event('selectionchange'));
    expect(input.selectionStart).toBe(0);
    expect(input.selectionEnd).toBe(input.value.length);
  });

  it('does not intercept the overlay toggle shortcut', () => {
    const key = new KeyboardEvent('keydown', { key: 'A', metaKey: true, shiftKey: true, bubbles: true, cancelable: true });
    canvas.dispatchEvent(key);
    expect(key.defaultPrevented).toBe(false);
  });

  it('blocks pointer selection on the canvas and removes its listeners on disposal', () => {
    const start = new Event('selectstart', { bubbles: true, cancelable: true });
    canvas.dispatchEvent(start);
    expect(start.defaultPrevented).toBe(true);
    stop();
    const afterDispose = new Event('selectstart', { bubbles: true, cancelable: true });
    canvas.dispatchEvent(afterDispose);
    expect(afterDispose.defaultPrevented).toBe(false);
  });

  it('clears a pre-existing selection when the protection is mounted', () => {
    stop();
    selectCanvas();
    stop = protectSelection();
    expect(document.getSelection()!.rangeCount).toBe(0);
  });
});
