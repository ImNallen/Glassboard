import { expect, it } from 'vitest';
import { getResourceUsage, usageLabel, type ResourceUsage } from './usage';

it.each<[ResourceUsage, string]>([
  [{ memoryBytes: 118 * 1024 ** 2, cpuPercent: 0.4 }, '118 MB · 0.4% CPU'],
  [{ memoryBytes: 118 * 1024 ** 2, cpuPercent: null }, '118 MB'],
  [{ memoryBytes: 1023 * 1024 ** 2, cpuPercent: 0 }, '1023 MB · 0.0% CPU'],
  [{ memoryBytes: 1023.6 * 1024 ** 2, cpuPercent: 0.04 }, '1.0 GB · 0.0% CPU'],
  [{ memoryBytes: 1024 ** 3, cpuPercent: 12.345 }, '1.0 GB · 12.3% CPU'],
  [{ memoryBytes: 1.56 * 1024 ** 3, cpuPercent: 150 }, '1.6 GB · 150.0% CPU'],
])('labels %o', (usage, label) => {
  expect(usageLabel(usage)).toBe(label);
});

it('measures nothing in the browser preview', async () => {
  expect(await getResourceUsage()).toBeNull();
});
