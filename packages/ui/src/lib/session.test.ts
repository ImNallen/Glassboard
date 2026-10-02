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

// Transitions themselves are covered by contract/transitions.json; this is the preview's own persistence.
describe('tutorial completion in the browser preview', () => {
  it('survives a reload once the tutorial is done', async () => {
    const app = await preview();
    await app.action('tutorial-start');
    await app.reportHistory({ canUndo: true, canRedo: false }, app.state().annotationSession);
    await app.action('toggle');
    expect(app.state().tutorial).toBe('done');
    vi.resetModules();
    const reloaded = await preview();
    expect(reloaded.state()).toMatchObject({ tutorial: null, mode: 'hidden', preferences: { tutorialCompleted: true } });
    await reloaded.action('replay-tutorial');
    expect(reloaded.state().tutorial).toBe('welcome');
  });

  it('survives a reload once the tutorial is skipped', async () => {
    const app = await preview();
    await app.action('dismiss-tutorial');
    vi.resetModules();
    expect((await preview()).state().tutorial).toBeNull();
  });
});
