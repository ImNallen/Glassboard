// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Session } from './session';

beforeEach(() => {
  // Node's optional global Storage can shadow jsdom's storage on recent Node releases.
  const values = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
  });
  vi.resetModules();
});

async function preview() {
  const api = await import('./session');
  let state: Session;
  await api.subscribe(value => state = value);
  return { ...api, state: () => state };
}

describe('annotation sessions and first launch', () => {
  it.each(['cancel-capture', 'hide', 'toggle', 'settings'])('isolates capture history and leaves work mode on %s', async exit => {
    const app = await preview();
    await app.action('show');
    const old = app.state().annotationSession;
    await app.action('capture');
    expect(app.state()).toMatchObject({ mode: 'hidden', activeOverlay: 'capture', capture: { id: old + 1, ready: true } });
    const generation = app.state().annotationSession;
    await app.action('capture');
    expect(app.state().annotationSession).toBe(generation);
    await app.reportHistory({ canUndo: true, canRedo: true }, old);
    expect(app.state().historyByOverlay).toEqual({});
    await app.reportHistory({ canUndo: true, canRedo: false }, generation);
    expect(app.state().historyByOverlay.capture.canUndo).toBe(true);
    await app.action(exit);
    expect(app.state()).toMatchObject({ mode: 'hidden', capture: null, historyByOverlay: {} });
    await app.reportHistory({ canUndo: true, canRedo: false }, generation);
    expect(app.state().historyByOverlay).toEqual({});
  });

  it('teaches drawing and returning to work, and remembers completion after reload', async () => {
    const app = await preview();
    expect(app.state().mode).toBe('hidden');
    expect(app.state().tutorial).toBe('welcome');
    await app.action('tutorial-start');
    expect(app.state().tutorial).toBe('draw');
    await app.reportHistory({ canUndo: true, canRedo: false }, app.state().annotationSession);
    expect(app.state().tutorial).toBe('hide');
    await app.action('toggle');
    expect(app.state().mode).toBe('hidden');
    expect(app.state().tutorial).toBe('done');
    vi.resetModules();
    const reloaded = await preview();
    expect(reloaded.state().tutorial).toBeNull();
    expect(reloaded.state().mode).toBe('hidden');
    await reloaded.action('replay-tutorial');
    expect(reloaded.state().tutorial).toBe('welcome');
  });

  it.each(['hide', 'toggle', 'replay-tutorial'])('clears on %s before reopening and rejects old history', async exit => {
    const app = await preview();
    await app.action('show');
    const first = app.state().annotationSession;
    await app.reportHistory({ canUndo: true, canRedo: true }, first);
    await app.action('show');
    expect(app.state().annotationSession).toBe(first);
    expect(app.state().historyByOverlay['overlay-0'].canUndo).toBe(true);
    await app.action(exit);
    expect(app.state().mode).toBe('hidden');
    expect(app.state().annotationSession).toBe(first + 1);
    expect(app.state().historyByOverlay).toEqual({});
    await app.reportHistory({ canUndo: true, canRedo: true }, first, true);
    expect(app.state().historyByOverlay).toEqual({});
    expect(app.state().cycleIndex).toBe(0);
    await app.action('hide');
    await app.action('show');
    expect(app.state().annotationSession).toBe(first + 1);
    expect(app.state().historyByOverlay).toEqual({});
    await expect(app.action('interact')).rejects.toThrow('Unknown action');
  });

  it('remembers skipping without entering annotation mode', async () => {
    const app = await preview();
    await app.action('dismiss-tutorial');
    expect(app.state().mode).toBe('hidden');
    vi.resetModules();
    expect((await preview()).state().tutorial).toBeNull();
  });
});
