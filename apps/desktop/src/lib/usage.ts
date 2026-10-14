import { invoke } from '@tauri-apps/api/core';
import { native } from '@glassboard/ui/session';

/** Memory and CPU used by Glassboard and its webview processes. CPU is a percent of one core, null on the first sample. */
export type ResourceUsage = { memoryBytes: number; cpuPercent: number | null };

/** The browser preview has no process to measure. */
export async function getResourceUsage() {
  return native ? invoke<ResourceUsage>('get_resource_usage') : null;
}

export function usageLabel({ memoryBytes, cpuPercent }: ResourceUsage) {
  const megabytes = Math.round(memoryBytes / 1024 ** 2);
  const memory = megabytes < 1024 ? `${megabytes} MB` : `${(memoryBytes / 1024 ** 3).toFixed(1)} GB`;
  return cpuPercent === null ? memory : `${memory} · ${cpuPercent.toFixed(1)}% CPU`;
}
