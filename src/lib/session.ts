import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { CYCLE_COLORS, type AutoFadeSeconds, type ColorMode, type Tool } from './drawing';
import { DEFAULT_SHORTCUT, formatShortcut } from './shortcuts';
export type ToolbarPosition = 'left' | 'right' | 'bottom';
export type Preferences = { tool: Tool; color: string; colorMode: ColorMode; width: number; shortcut: string; toolbarPosition: ToolbarPosition; autoFadeSeconds: AutoFadeSeconds };
export type HistoryAvailability = { canUndo: boolean; canRedo: boolean };
export type Session = { mode: 'hidden' | 'draw' | 'interact'; settingsOpen: boolean; activeOverlay: string; cycleIndex: number; historyByOverlay: Record<string, HistoryAvailability>; preferences: Preferences; error: string | null };
export const native = isTauri();
export const defaults: Session = { mode: 'draw', settingsOpen: false, activeOverlay: 'overlay-0', cycleIndex: 0, historyByOverlay: {}, preferences: { tool: 'arrow', color: CYCLE_COLORS[0], colorMode: 'rainbow', width: 4, shortcut: DEFAULT_SHORTCUT, toolbarPosition: 'bottom', autoFadeSeconds: 0 }, error: null };
let preview = structuredClone(defaults);
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
  if (['undo', 'redo', 'clear', 'clear-all', 'erase'].includes(action)) { drawingSubscribers.forEach(fn => fn(action === 'clear-all' ? 'clear' : action)); return; }
  if (action === 'toggle') preview.mode = preview.mode === 'hidden' ? 'draw' : 'hidden';
  if (action === 'show') preview.mode = 'draw';
  if (action === 'hide') preview.mode = 'hidden';
  if (action === 'interact' && preview.mode !== 'hidden') preview.mode = preview.mode === 'interact' ? 'draw' : 'interact';
  if (action === 'settings') preview.settingsOpen = true;
  if (['toggle', 'show', 'hide', 'close-settings'].includes(action)) preview.settingsOpen = false;
  if (action === 'dismiss-error') preview.error = null;
  publish();
}
export async function savePreferences(preferences: Preferences) {
  if (native) return invoke<void>('set_preferences', { preferences });
  preview.preferences = preferences; publish();
}
/** Native cursor proximity to the docked toolbar. The browser preview measures this in the DOM instead. */
export async function toolbarProximity(fn: (near: boolean) => void) {
  if (native) return listen<boolean>('toolbar-proximity', event => fn(event.payload));
  return () => {};
}
export async function activateOverlay() { if (native) await invoke('activate_overlay'); }
export async function reportHistory(availability: HistoryAvailability, advanceCycle = false) {
  if (native) return invoke<void>('report_history', { availability, advanceCycle });
  if (advanceCycle) preview.cycleIndex++;
  preview.historyByOverlay['overlay-0'] = availability;
  publish();
}
export async function expandToolbar(expanded: boolean) { if (native) await invoke('expand_toolbar', { expanded }); }
export const mac = navigator.platform.toLowerCase().includes('mac');
export function shortcutLabel(shortcut: string) { return formatShortcut(shortcut, mac); }
