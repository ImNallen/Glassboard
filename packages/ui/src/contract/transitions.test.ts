import { describe, expect, it } from 'vitest';
import cases from './transitions.json';
import { defaults, type Action, type HistoryAvailability, type Session } from '../lib/session-model';
import { createPreviewSession } from '../lib/session-preview';

type Step = Action | { history: HistoryAvailability; annotationSession: number; advanceCycle?: boolean };
type Case = { name: string; from?: Partial<Omit<Session, 'preferences'>> & { preferences?: Partial<Session['preferences']> }; steps: Step[]; expect: unknown };

const isObject = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);

/** `expect` names only the fields it checks. An empty object requires an empty one, and `{"$rust": …, "$preview": …}` picks this side's value. */
function mismatch(actual: unknown, expected: unknown, path: string): string | undefined {
  if (!isObject(expected)) return JSON.stringify(actual) === JSON.stringify(expected) ? undefined : `${path}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`;
  if (Object.keys(expected).some(key => key.startsWith('$'))) return '$preview' in expected ? mismatch(actual, expected.$preview, path) : `${path}: no $preview value`;
  if (!isObject(actual)) return `${path}: expected an object, got ${JSON.stringify(actual)}`;
  if (Object.keys(expected).length === 0 && Object.keys(actual).length > 0) return `${path}: expected {}, got ${JSON.stringify(actual)}`;
  for (const key of Object.keys(expected)) {
    const problem = mismatch(actual[key], expected[key], `${path}.${key}`);
    if (problem) return problem;
  }
}

describe('the preview session follows the shared transition fixture', () => {
  it.each(cases as Case[])('$name', async ({ from = {}, steps, expect: expected }) => {
    const session = createPreviewSession({ ...from, preferences: { ...defaults.preferences, ...from.preferences } });
    let state!: Session;
    await session.subscribe(value => state = value);
    for (const step of steps) {
      if (typeof step === 'string') await session.action(step);
      else await session.reportHistory(step.history, step.annotationSession, step.advanceCycle ?? false);
    }
    expect(mismatch(state, expected, 'session')).toBeUndefined();
  });
});
