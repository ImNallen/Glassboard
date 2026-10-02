import type { HistoryAvailability, SessionBackend } from './session-model';
import { createPreviewSession } from './session-preview';
export * from './session-model';
export { createPreviewSession };

let backend = createPreviewSession();
/** True once the desktop app connects its Tauri backend. Read it at render or call time, never at module load. */
export let native = false;
/** Pick the backend before mounting: the desktop app connects Tauri, and tests reset to a fresh preview between cases. Hosts that never call it run the browser preview. */
export function useSession(next = createPreviewSession(), options = { native: false }) { backend = next; native = options.native; }
export const subscribe: SessionBackend['subscribe'] = fn => backend.subscribe(fn);
export const drawingEvents: SessionBackend['drawingEvents'] = fn => backend.drawingEvents(fn);
export const action: SessionBackend['action'] = name => backend.action(name);
export const savePreferences: SessionBackend['savePreferences'] = preferences => backend.savePreferences(preferences);
export function reportHistory(availability: HistoryAvailability, annotationSession: number, advanceCycle = false) { return backend.reportHistory(availability, annotationSession, advanceCycle); }
export const activateOverlay: SessionBackend['activateOverlay'] = () => backend.activateOverlay();
export const expandToolbar: SessionBackend['expandToolbar'] = expanded => backend.expandToolbar(expanded);
export const toolbarPointer: SessionBackend['toolbarPointer'] = fn => backend.toolbarPointer(fn);
