import { defaults, type Session, type SessionBackend, type Transition } from './session-model';

const COMPLETED_KEY = 'glassboard-tutorial-completed';
/** Mirrors `Transition::leaves_capture` in session.rs. */
const LEAVES_CAPTURE: readonly Transition[] = ['toggle', 'show', 'hide', 'cancel-capture', 'tutorial-start', 'replay-tutorial', 'settings'];

/** The browser stand-in for the Rust session state machine, starting from defaults overridden by `initial`. */
export function createPreviewSession(initial: Partial<Session> = {}): SessionBackend {
  const preview: Session = structuredClone({ ...defaults, ...initial });
  try { if (localStorage.getItem(COMPLETED_KEY) === 'true') preview.preferences.tutorialCompleted = true; }
  catch { /* Storage may be unavailable in a browser preview. */ }
  if (initial.tutorial === undefined && preview.preferences.tutorialCompleted) preview.tutorial = null;
  const subscribers = new Set<(session: Session) => void>();
  const drawingSubscribers = new Set<(action: string) => void>();
  function publish() { subscribers.forEach(fn => fn(structuredClone(preview))); }
  // The desktop app saves this through `commands::transition`; the preview keeps it in the browser.
  function completeTutorial() {
    preview.preferences.tutorialCompleted = true;
    try { localStorage.setItem(COMPLETED_KEY, 'true'); } catch { /* Preview only. */ }
  }
  function beginCapture() {
    if (preview.capture) return;
    preview.annotationSession++;
    preview.historyByOverlay = {};
    preview.mode = 'hidden';
    preview.settingsOpen = false;
    if (preview.tutorial === 'draw' || preview.tutorial === 'hide') preview.tutorial = 'welcome';
    preview.activeOverlay = 'capture';
    preview.error = null;
    preview.capture = { id: preview.annotationSession, ready: true };
  }
  function transition(action: Transition) {
    const previousMode = preview.mode;
    const leavingCapture = Boolean(preview.capture) && LEAVES_CAPTURE.includes(action);
    switch (action) {
      case 'toggle': preview.mode = preview.mode === 'hidden' && !preview.capture ? 'draw' : 'hidden'; break;
      case 'show': preview.mode = 'draw'; break;
      case 'hide': case 'cancel-capture': preview.mode = 'hidden'; break;
      case 'tutorial-start': preview.mode = 'draw'; preview.tutorial = 'draw'; break;
      case 'replay-tutorial': preview.mode = 'hidden'; preview.tutorial = 'welcome'; break;
      case 'dismiss-tutorial': preview.tutorial = null; completeTutorial(); break;
      case 'settings': preview.settingsOpen = true; break;
      case 'close-settings': preview.settingsOpen = false; break;
      case 'dismiss-error': preview.error = null; break;
    }
    if (leavingCapture) preview.capture = null;
    if (leavingCapture || (previousMode === 'draw' && preview.mode === 'hidden')) {
      preview.annotationSession++;
      preview.historyByOverlay = {};
    }
    if (previousMode === 'hidden' && preview.mode === 'draw' && preview.tutorial === 'welcome') preview.tutorial = 'draw';
    if (previousMode === 'draw' && preview.mode === 'hidden' && action !== 'replay-tutorial') {
      if (preview.tutorial === 'hide') { preview.tutorial = 'done'; completeTutorial(); }
      else if (preview.tutorial === 'draw') preview.tutorial = 'welcome';
    }
    if (LEAVES_CAPTURE.includes(action) && action !== 'settings') preview.settingsOpen = false;
  }

  return {
    async subscribe(fn) {
      subscribers.add(fn); fn(structuredClone(preview));
      return () => { subscribers.delete(fn); };
    },
    async drawingEvents(fn) {
      drawingSubscribers.add(fn); return () => { drawingSubscribers.delete(fn); };
    },
    async action(action) {
      switch (action) {
        case 'undo': case 'redo': case 'clear': drawingSubscribers.forEach(fn => fn(action)); return;
        case 'clear-all': drawingSubscribers.forEach(fn => fn('clear')); return;
        case 'capture': beginCapture(); break;
        case 'open-github': case 'report-issue': case 'check-for-updates': case 'install-update': case 'quit': throw new Error(`${action} needs the desktop app`);
        default: transition(action);
      }
      publish();
    },
    async savePreferences(preferences) {
      // Copy nested values so a Svelte state proxy never reaches structuredClone.
      preview.preferences = { ...preferences, keybindings: { ...preferences.keybindings }, swatches: [...preferences.swatches], rainbowColors: [...preferences.rainbowColors], cycleColors: [...preferences.cycleColors] }; publish();
    },
    async reportHistory(availability, annotationSession, advanceCycle) {
      if (annotationSession !== preview.annotationSession) return;
      if (preview.tutorial === 'draw' && availability.canUndo) preview.tutorial = 'hide';
      if (advanceCycle) preview.cycleIndex++;
      preview.historyByOverlay[preview.activeOverlay] = availability;
      publish();
    },
    async activateOverlay() {},
    async expandToolbar() {},
    async toolbarPointer() { return () => {}; },
  };
}
