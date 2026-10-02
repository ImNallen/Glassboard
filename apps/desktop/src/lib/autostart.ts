import { invoke } from '@tauri-apps/api/core';
import { native } from '@glassboard/ui/session';

// The OS owns the login item; the browser preview keeps a stand-in.
let previewAutostart = false;
/** Whether Glassboard opens at login. */
export async function getAutostart() {
  return native ? invoke<boolean>('get_autostart') : previewAutostart;
}
/** Turn opening at login on or off, returning the state the OS now reports. */
export async function setAutostart(enabled: boolean) {
  return native ? invoke<boolean>('set_autostart', { enabled }) : previewAutostart = enabled;
}
