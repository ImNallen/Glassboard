import preferenceDefaults from '../contract/preference-defaults.json';
import type { AutoFadeSeconds, ColorMode, Tool } from './drawing';
import type { Keybindings } from './shortcuts';
export type ToolbarPosition = 'left' | 'right' | 'bottom';
export type ToolbarDock = { x: number; y: number; width: number; height: number };
export type Preferences = { tool: Tool; color: string; colorMode: ColorMode; shortcut: string; keybindings: Keybindings; swatches: string[]; rainbowColors: string[]; cycleColors: string[]; toolbarPosition: ToolbarPosition; autoFadeSeconds: AutoFadeSeconds; tutorialCompleted: boolean };
/** Rust loads the same file. JSON widens literals, so the check here is the shape; the Rust parse of the file rejects a bad literal. */
const widened = preferenceDefaults satisfies { [K in keyof Preferences]: Preferences[K] extends string ? string : Preferences[K] extends number ? number : Preferences[K] };
export const defaultPreferences = widened as Preferences;
export type HistoryAvailability = { canUndo: boolean; canRedo: boolean };
export type Session = { mode: 'hidden' | 'draw'; annotationSession: number; tutorial: 'welcome' | 'draw' | 'hide' | 'done' | null; settingsOpen: boolean; activeOverlay: string; cycleIndex: number; historyByOverlay: Record<string, HistoryAvailability>; preferences: Preferences; error: string | null; shortcutUnavailable: boolean; capture: { id: number; ready: boolean; toolbarDock?: ToolbarDock } | null };
export type ToolbarPointer = { near: boolean; x: number; y: number };
/** Actions the session state machine owns; `contract/transitions.json` replays each on both sides. */
export type Transition = 'toggle' | 'show' | 'hide' | 'cancel-capture' | 'tutorial-start' | 'replay-tutorial' | 'dismiss-tutorial' | 'settings' | 'close-settings' | 'dismiss-error';
/** The `action` command vocabulary, matching Rust's `commands::Action`. */
export type Action = Transition | 'capture' | 'undo' | 'redo' | 'clear' | 'clear-all' | 'open-github' | 'report-issue' | 'check-for-updates' | 'install-update' | 'quit';
export const defaults: Session = { mode: 'hidden', annotationSession: 0, tutorial: 'welcome', settingsOpen: false, activeOverlay: 'overlay-0', cycleIndex: 0, historyByOverlay: {}, capture: null, preferences: defaultPreferences, error: null, shortcutUnavailable: false };
type Unsubscribe = () => void;
/** What every surface needs from the app: the desktop implements it over Tauri IPC, the browser preview in memory. */
export type SessionBackend = {
  subscribe(fn: (session: Session) => void): Promise<Unsubscribe>;
  drawingEvents(fn: (action: string) => void): Promise<Unsubscribe>;
  action(action: Action): Promise<void>;
  savePreferences(preferences: Preferences): Promise<void>;
  reportHistory(availability: HistoryAvailability, annotationSession: number, advanceCycle: boolean): Promise<void>;
  activateOverlay(): Promise<void>;
  expandToolbar(expanded: boolean): Promise<void>;
  /** Focus-independent cursor samples in toolbar CSS pixels; null means the cursor left. */
  toolbarPointer(fn: (pointer: ToolbarPointer | null) => void): Promise<Unsubscribe>;
};
