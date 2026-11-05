import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { number, t } from '@glassboard/ui/i18n';
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
    case 'disabled': return t('Automatic updates are unavailable in this build.');
    case 'idle': return t('Glassboard checks GitHub for new versions once a day.');
    case 'checking': return t('Checking…');
    case 'downloading': return status.percent === null
      ? t('Downloading version {version}…', { version: status.version })
      : t('Downloading version {version}… {percent}', { version: status.version, percent: number(status.percent / 100, { style: 'percent' }) });
    case 'ready': return t('Version {version} is ready.', { version: status.version });
    case 'installing': return t('Installing version {version}…', { version: status.version });
    case 'failed': return status.message;
  }
}
export function updateLabel(status: UpdateStatus) {
  switch (status.state) {
    case 'ready': return t('Restart to install version {version}', { version: status.version });
    case 'checking': case 'downloading': case 'installing': return updateDetail(status);
    default: return t('Check for updates');
  }
}
