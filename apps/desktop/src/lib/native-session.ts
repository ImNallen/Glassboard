import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Session, SessionBackend, ToolbarPointer } from '@glassboard/ui/session';

export const nativeSession: SessionBackend = {
  async subscribe(fn) {
    // Subscribe first so a transition during startup cannot be lost.
    let received = false;
    const unlisten = await listen<Session>('session', event => { received = true; fn(event.payload); });
    try { const initial = await invoke<Session>('get_session'); if (!received) fn(initial); }
    catch (error) { unlisten(); throw error; }
    return unlisten;
  },
  drawingEvents: fn => listen<string>('drawing-action', event => fn(event.payload)),
  action: action => invoke('action', { action }),
  savePreferences: preferences => invoke('set_preferences', { preferences }),
  reportHistory: (availability, annotationSession, advanceCycle) => invoke('report_history', { availability, annotationSession, advanceCycle }),
  activateOverlay: () => invoke('activate_overlay'),
  expandToolbar: expanded => invoke('expand_toolbar', { expanded }),
  toolbarPointer: fn => listen<ToolbarPointer | null>('toolbar-pointer', event => fn(event.payload)),
};
