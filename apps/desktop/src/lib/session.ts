import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { CYCLE_COLORS, type AutoFadeSeconds, type ColorMode, type Tool } from './drawing';
import { DEFAULT_SHORTCUT, formatShortcut } from './shortcuts';
export type ToolbarPosition = 'left' | 'right' | 'bottom';
export type Preferences = { tool: Tool; color: string; colorMode: ColorMode; shortcut: string; toolbarPosition: ToolbarPosition; autoFadeSeconds: AutoFadeSeconds; tutorialCompleted: boolean };
export type HistoryAvailability = { canUndo: boolean; canRedo: boolean };
export type Session = { mode: 'hidden' | 'draw'; annotationSession: number; tutorial: 'welcome' | 'draw' | 'hide' | 'done' | null; settingsOpen: boolean; activeOverlay: string; cycleIndex: number; historyByOverlay: Record<string, HistoryAvailability>; preferences: Preferences; error: string | null };
export const native = isTauri();
export const defaults: Session = { mode: 'hidden', annotationSession: 0, tutorial: 'welcome', settingsOpen: false, activeOverlay: 'overlay-0', cycleIndex: 0, historyByOverlay: {}, preferences: { tool: 'arrow', color: CYCLE_COLORS[0], colorMode: 'rainbow', shortcut: DEFAULT_SHORTCUT, toolbarPosition: 'bottom', autoFadeSeconds: 0, tutorialCompleted: false }, error: null };
let preview = structuredClone(defaults);
try {
  if (localStorage.getItem('glassboard-tutorial-completed') === 'true') {
    preview.preferences.tutorialCompleted = true;
    preview.tutorial = null;
  }
} catch { /* Storage may be unavailable in a browser preview. */ }
const subscribers = new Set<(session: Session) => void>();
const drawingSubscribers = new Set<(action: string) => void>();
function publish() { subscribers.forEach(fn => fn(structuredClone(preview))); }
export async function subscribe(fn: (session: Session) => void) {
  if (native) {
    // Subscribe first so a transition during startup cannot be lost.
    let received = false;
    const unlisten = await listen<Session>('session', event => { received = true; fn(event.payload); });
    try { const initial = await invoke<Session>('get_session'); if (!received) fn(initial); }
    catch (error) { unlisten(); throw error; }
    return unlisten;
  }
  subscribers.add(fn); fn(structuredClone(preview));
  return () => { subscribers.delete(fn); };
}
export async function drawingEvents(fn: (action: string) => void) {
  if (native) return listen<string>('drawing-action', event => fn(event.payload));
  drawingSubscribers.add(fn); return () => { drawingSubscribers.delete(fn); };
}
export async function action(action: string) {
  if (native) return invoke<void>('action', { action });
  if (['undo', 'redo', 'clear', 'clear-all'].includes(action)) { drawingSubscribers.forEach(fn => fn(action === 'clear-all' ? 'clear' : action)); return; }
  if (!['toggle', 'show', 'hide', 'tutorial-start', 'replay-tutorial', 'dismiss-tutorial', 'settings', 'close-settings', 'dismiss-error'].includes(action)) throw new Error('Unknown action');
  const previousMode = preview.mode;
  if (action === 'dismiss-tutorial') {
    try { localStorage.setItem('glassboard-tutorial-completed', 'true'); } catch { /* Preview only. */ }
    preview.preferences.tutorialCompleted = true;
    preview.tutorial = null;
  }
  if (action === 'replay-tutorial') { preview.mode = 'hidden'; preview.tutorial = 'welcome'; preview.settingsOpen = false; }
  if (action === 'tutorial-start') { preview.mode = 'draw'; preview.tutorial = 'draw'; }
  if (action === 'toggle') preview.mode = preview.mode === 'hidden' ? 'draw' : 'hidden';
  if (action === 'show') preview.mode = 'draw';
  if (action === 'hide') preview.mode = 'hidden';
  if (previousMode === 'draw' && preview.mode === 'hidden') {
    preview.annotationSession++;
    preview.historyByOverlay = {};
  }
  if (previousMode === 'hidden' && preview.mode === 'draw') {
    if (preview.tutorial === 'welcome') preview.tutorial = 'draw';
  }
  if (previousMode === 'draw' && preview.mode === 'hidden' && action !== 'replay-tutorial') {
    if (preview.tutorial === 'hide') {
      preview.tutorial = 'done';
      preview.preferences.tutorialCompleted = true;
      try { localStorage.setItem('glassboard-tutorial-completed', 'true'); } catch { /* Preview only. */ }
    }
    else if (preview.tutorial === 'draw') preview.tutorial = 'welcome';
  }
  if (action === 'settings') preview.settingsOpen = true;
  if (['toggle', 'show', 'hide', 'tutorial-start', 'close-settings'].includes(action)) preview.settingsOpen = false;
  if (action === 'dismiss-error') preview.error = null;
  publish();
}
export async function savePreferences(preferences: Preferences) {
  if (native) return invoke<void>('set_preferences', { preferences });
  preview.preferences = preferences; publish();
}
export type ToolbarPointer = { near: boolean; x: number; y: number };
/** Focus-independent cursor samples in toolbar CSS pixels; null means the cursor left. */
export async function toolbarPointer(fn: (pointer: ToolbarPointer | null) => void) {
  if (native) return listen<ToolbarPointer | null>('toolbar-pointer', event => fn(event.payload));
  return () => {};
}
export async function activateOverlay() { if (native) await invoke('activate_overlay'); }
export async function reportHistory(availability: HistoryAvailability, annotationSession: number, advanceCycle = false) {
  if (native) return invoke<void>('report_history', { availability, annotationSession, advanceCycle });
  if (annotationSession !== preview.annotationSession) return;
  if (preview.tutorial === 'draw' && availability.canUndo) preview.tutorial = 'hide';
  if (advanceCycle) preview.cycleIndex++;
  preview.historyByOverlay['overlay-0'] = availability;
  publish();
}
export async function expandToolbar(expanded: boolean) { if (native) await invoke('expand_toolbar', { expanded }); }
export const mac = navigator.platform.toLowerCase().includes('mac');
export function shortcutLabel(shortcut: string) { return formatShortcut(shortcut, mac); }
