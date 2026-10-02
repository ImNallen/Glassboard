import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { createErrors } from './errors.svelte';
import { action, defaults, type Session } from './session';

vi.mock('./session', async importOriginal => ({ ...await importOriginal<typeof import('./session')>(), action: vi.fn().mockResolvedValue(undefined) }));

let log: ReturnType<typeof vi.spyOn>;
beforeEach(() => { vi.useFakeTimers(); vi.mocked(action).mockClear(); log = vi.spyOn(console, 'error').mockImplementation(() => {}); });
afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks(); });

const session = (error: string | null): Session => ({ ...structuredClone(defaults), error });

it('shows a reported error over the session error, logs it once, and clears it after 8 seconds', () => {
  const errors = createErrors(() => session('Some settings could not be read.'));
  errors.report(new Error('Clipboard busy'));
  expect(errors.message).toBe('Error: Clipboard busy');
  expect(log).toHaveBeenCalledOnce();
  vi.advanceTimersByTime(7999);
  expect(errors.message).toBe('Error: Clipboard busy');
  vi.advanceTimersByTime(1);
  expect(errors.message).toBe('Some settings could not be read.');
});

it('clears without logging or leaving a timer behind', () => {
  const errors = createErrors(() => session(null));
  errors.report('Clipboard busy');
  log.mockClear();
  errors.clear();
  expect(errors.message).toBe('');
  expect(log).not.toHaveBeenCalled();
  expect(vi.getTimerCount()).toBe(0);
});

it('dismisses the session error in the backend only when the session has one', () => {
  let current = session('Some settings could not be read.');
  const errors = createErrors(() => current);
  errors.report('Clipboard busy');
  errors.dismiss();
  expect(errors.message).toBe('Some settings could not be read.');
  expect(action).toHaveBeenCalledExactlyOnceWith('dismiss-error');

  vi.mocked(action).mockClear();
  current = session(null);
  errors.report('Clipboard busy');
  errors.dismiss();
  expect(errors.message).toBe('');
  expect(action).not.toHaveBeenCalled();
});
