import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { CYCLE_COLORS, type AutoFadeSeconds, type ColorMode, type Tool } from './drawing';
export type ToolbarPosition = 'left' | 'right' | 'bottom';
export type Preferences = { tool: Tool; color: string; colorMode: ColorMode; width: number; shortcut: string; toolbarPosition: ToolbarPosition; autoFadeSeconds: AutoFadeSeconds };
export type HistoryAvailability = { canUndo: boolean; canRedo: boolean };
export type Session = { mode: 'hidden' | 'draw' | 'interact'; toolbarVisible: boolean; settingsOpen: boolean; activeOverlay: string; cycleIndex: number; historyByOverlay: Record<string, HistoryAvailability>; preferences: Preferences; error: string | null };
export const native = isTauri();
export const defaults: Session = { mode: 'draw', toolbarVisible: true, settingsOpen: false, activeOverlay: 'overlay-0', cycleIndex: 0, historyByOverlay: {}, preferences: { tool: 'arrow', color: CYCLE_COLORS[0], colorMode: 'rainbow', width: 4, shortcut: 'CommandOrControl+Shift+A', toolbarPosition: 'bottom', autoFadeSeconds: 0 }, error: null };
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
export async function action(action: string, keepSettingsOpen = false) {
  // Native commands identify the originating window; preview mirrors that behavior.
  if (native) return invoke<void>('action', { action });
  const wasSettingsOpen = preview.settingsOpen;
  if (['undo', 'redo', 'clear', 'clear-all', 'erase'].includes(action)) { drawingSubscribers.forEach(fn => fn(action === 'clear-all' ? 'clear' : action)); return; }
  if (action === 'toggle') { preview.mode = preview.mode === 'hidden' ? 'draw' : 'hidden'; preview.toolbarVisible = true; }
  if (action === 'show') { preview.mode = 'draw'; preview.toolbarVisible = true; }
  if (action === 'hide') preview.mode = 'hidden';
  if (action === 'interact' && preview.mode !== 'hidden') preview.mode = preview.mode === 'interact' ? 'draw' : 'interact';
  if (action === 'toolbar' && preview.mode !== 'hidden') preview.toolbarVisible = !preview.toolbarVisible;
  if (action === 'settings') preview.settingsOpen = true;
  if (['toggle', 'show', 'hide', 'toolbar', 'close-settings'].includes(action)) preview.settingsOpen = false;
  if (keepSettingsOpen && ['toggle', 'show', 'hide', 'interact', 'toolbar'].includes(action)) preview.settingsOpen = wasSettingsOpen;
  if (action === 'dismiss-error') preview.error = null;
  publish();
}
export async function savePreferences(preferences: Preferences) {
  if (native) return invoke<void>('set_preferences', { preferences });
  preview.preferences = preferences; publish();
}
export async function activateOverlay() { if (native) await invoke('activate_overlay'); }
export async function reportHistory(availability: HistoryAvailability, advanceCycle = false) {
  if (native) return invoke<void>('report_history', { availability, advanceCycle });
  if (advanceCycle) preview.cycleIndex++;
  preview.historyByOverlay['overlay-0'] = availability;
  publish();
}
export async function expandToolbar(expanded: boolean) { if (native) await invoke('expand_toolbar', { expanded }); }
export function shortcutLabel(shortcut: string) {
  const mac = navigator.platform.toLowerCase().includes('mac');
  return shortcut.replace(/CommandOrControl/gi, mac ? '⌘' : 'Ctrl').replace(/Shift/gi, mac ? '⇧' : 'Shift').replace(/Alt/gi, mac ? '⌥' : 'Alt').replaceAll('+', mac ? '' : '+');
}
