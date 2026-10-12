import { invoke } from '@tauri-apps/api/core';
import { native } from '@glassboard/ui/session';

/** Release the system-wide shortcuts while recording one, since the OS performs a held combination before the webview sees it. */
export async function suspendGlobalShortcuts(suspended: boolean) {
  if (native) await invoke<void>('suspend_global_shortcuts', { suspended });
}
