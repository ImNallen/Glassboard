import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { action, native } from '@glassboard/ui/session';

export type UpdateStatus =
  | { state: 'disabled' } | { state: 'idle' } | { state: 'checking' }
  | { state: 'downloading'; version: string; percent: number | null } | { state: 'ready'; version: string }
  | { state: 'installing'; version: string } | { state: 'failed'; message: string };

const previewWatchers = new Set<(status: UpdateStatus) => void>();

export async function watchUpdates(fn: (status: UpdateStatus) => void): Promise<() => void> {
  if (!native) {
    previewWatchers.add(fn); fn({ state: 'idle' });
    return () => { previewWatchers.delete(fn); };
  }
  let received = false;
  const unlisten = await listen<UpdateStatus>('update', event => { received = true; fn(event.payload); });
  try { const initial = await invoke<UpdateStatus>('get_update'); if (!received) fn(initial); }
  catch (error) { unlisten(); throw error; }
  return unlisten;
}
export async function checkForUpdates() {
  if (native) return action('check-for-updates');
  for (const state of ['checking', 'idle'] as const) previewWatchers.forEach(fn => fn({ state }));
}
export const installUpdate = () => action('install-update');

export function updateDetail(status: UpdateStatus) {
  switch (status.state) {
    case 'disabled': return 'Automatic updates are unavailable in this build.';
    case 'idle': return 'Glassboard checks GitHub for new versions once a day.';
    case 'checking': return 'Checking…';
    case 'downloading': return `Downloading version ${status.version}…${status.percent === null ? '' : ` ${status.percent}%`}`;
    case 'ready': return `Version ${status.version} is ready.`;
    case 'installing': return `Installing version ${status.version}…`;
    case 'failed': return status.message;
  }
}
export function updateLabel(status: UpdateStatus) {
  switch (status.state) {
    case 'ready': return `Restart to install version ${status.version}`;
    case 'checking': case 'downloading': case 'installing': return updateDetail(status);
    default: return 'Check for updates';
  }
}
