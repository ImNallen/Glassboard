import { action, type Session } from './session';

export type Errors = ReturnType<typeof createErrors>;

/** One host's error line: a failure from this webview, which clears itself after a while, shown over the session's error from the backend. */
export function createErrors(session: () => Session) {
  let local = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;
  function clear() { local = ''; clearTimeout(timer); }
  function report(error: unknown) { console.error(error); local = String(error); clearTimeout(timer); timer = setTimeout(clear, 8000); }
  return {
    get message() { return local || session().error || ''; },
    /** Only this host's own failure, without the backend's. */
    get reported() { return local; },
    report, clear,
    dismiss() { clear(); if (session().error) action('dismiss-error').catch(report); },
  };
}
