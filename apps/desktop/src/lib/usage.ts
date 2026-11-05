import { invoke } from '@tauri-apps/api/core';
import { number, t } from '@glassboard/ui/i18n';
import { native } from '@glassboard/ui/session';

/** Memory and CPU used by Glassboard and its webview processes. CPU is a percent of one core, null on the first sample. */
export type ResourceUsage = { memoryBytes: number; cpuPercent: number | null };

/** The browser preview has no process to measure. */
export async function getResourceUsage() {
  return native ? invoke<ResourceUsage>('get_resource_usage') : null;
}

const ONE_DECIMAL = { minimumFractionDigits: 1, maximumFractionDigits: 1 };

export function usageLabel({ memoryBytes, cpuPercent }: ResourceUsage) {
  const megabytes = Math.round(memoryBytes / 1024 ** 2);
  const memory = megabytes < 1024
    ? number(megabytes, { style: 'unit', unit: 'megabyte', useGrouping: false })
    : number(memoryBytes / 1024 ** 3, { style: 'unit', unit: 'gigabyte', ...ONE_DECIMAL });
  return cpuPercent === null ? memory : t('{memory} · {percent} CPU', { memory, percent: number(cpuPercent / 100, { style: 'percent', ...ONE_DECIMAL }) });
}
