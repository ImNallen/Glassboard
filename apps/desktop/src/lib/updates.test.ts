import { expect, it } from 'vitest';
import { checkForUpdates, installUpdate, updateDetail, updateLabel, watchUpdates, type UpdateStatus } from './updates';

it.each<[UpdateStatus, string]>([
  [{ state: 'disabled' }, 'Updates come with release builds.'],
  [{ state: 'idle' }, 'Glassboard checks GitHub for new versions once a day.'],
  [{ state: 'checking' }, 'Checking…'],
  [{ state: 'downloading', version: '0.2.0', percent: null }, 'Downloading version 0.2.0…'],
  [{ state: 'downloading', version: '0.2.0', percent: 42 }, 'Downloading version 0.2.0… 42%'],
  [{ state: 'ready', version: '0.2.0' }, 'Version 0.2.0 is ready.'],
  [{ state: 'installing', version: '0.2.0' }, 'Installing version 0.2.0…'],
  [{ state: 'failed', message: "Couldn't reach GitHub to check for updates." }, "Couldn't reach GitHub to check for updates."],
])('describes %o', (status, detail) => {
  expect(updateDetail(status)).toBe(detail);
});

it.each<[UpdateStatus, string]>([
  [{ state: 'idle' }, 'Check for updates'],
  [{ state: 'failed', message: 'offline' }, 'Check for updates'],
  [{ state: 'checking' }, 'Checking…'],
  [{ state: 'downloading', version: '0.2.0', percent: 42 }, 'Downloading version 0.2.0… 42%'],
  [{ state: 'ready', version: '0.2.0' }, 'Restart to install version 0.2.0'],
  [{ state: 'installing', version: '0.2.0' }, 'Installing version 0.2.0…'],
])('labels the header button for %o', (status, label) => {
  expect(updateLabel(status)).toBe(label);
});

it('checks the browser preview back to idle and cannot install there', async () => {
  const seen: UpdateStatus[] = [];
  const stop = await watchUpdates(status => seen.push(status));
  await checkForUpdates();
  stop();
  await checkForUpdates();
  expect(seen).toEqual([{ state: 'idle' }, { state: 'checking' }, { state: 'idle' }]);
  await expect(installUpdate()).rejects.toThrow('install-update needs the desktop app');
});
