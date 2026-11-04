// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount, type Component } from 'svelte';
import { useCatalogs, setLocale, locale, t } from '@glassboard/ui/i18n';
import Capture from '@glassboard/ui/Capture.svelte';
import Overlay from '@glassboard/ui/Overlay.svelte';
import Toolbar from '@glassboard/ui/Toolbar.svelte';
import { createErrors } from '@glassboard/ui/errors';
import { createPreviewSession, defaults, savePreferences, subscribe, useSession, type Preferences, type Session, type SessionBackend } from '@glassboard/ui/session';
import Settings from './Settings.svelte';
import Tutorial from './Tutorial.svelte';
import { pseudo, pseudoCatalog } from './lib/pseudo';
import { watchUpdates, type UpdateStatus } from './lib/updates';
import { getResourceUsage } from './lib/usage';

vi.mock('./lib/updates', async original => {
  const updates = await original<typeof import('./lib/updates')>();
  return { ...updates, watchUpdates: vi.fn(updates.watchUpdates) };
});
vi.mock('./lib/usage', async original => {
  const usage = await original<typeof import('./lib/usage')>();
  return { ...usage, getResourceUsage: vi.fn(usage.getResourceUsage) };
});
vi.mock('@glassboard/ui/drawing', async original => ({ ...await original<typeof import('@glassboard/ui/drawing')>(), render: vi.fn() }));

useCatalogs({ pseudo: pseudoCatalog() });
const VISIBLE_ATTRIBUTES = ['aria-label', 'title', 'placeholder', 'aria-valuetext'];
/** What stays unwrapped on purpose: the brand, color codes, and the units and list words Intl supplies, which a pseudo language can't reach. */
const ALLOWED = [/⟦[^⟧]*⟧/g, /Glassboard/g, /#[0-9a-f]{3,8}\b/gi, /\b\d+ (seconds?|MB|GB)\b/g, /\b(and|or)\b/g];

function leaks(root: Element = document.body) {
  const found: string[] = [];
  const check = (where: string, text: string) => {
    if (/\p{L}{2,}/u.test(ALLOWED.reduce((rest, allowed) => rest.replace(allowed, ''), text))) found.push(`${where}: ${text}`);
  };
  const nested = (element: Element) => {
    const language = element.closest('[lang]');
    return language && language !== document.documentElement;
  };
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (node instanceof Text) { if (node.parentElement && !nested(node.parentElement)) check('text', node.data); continue; }
    const element = node as Element;
    if (nested(element)) continue;
    for (const attribute of VISIBLE_ATTRIBUTES) if (element.hasAttribute(attribute)) check(attribute, element.getAttribute(attribute)!);
  }
  return found;
}

let mounted: ReturnType<typeof mount>[] = [];
let stops: (() => void)[] = [];
afterEach(async () => {
  for (const component of mounted) await unmount(component);
  mounted = [];
  stops.forEach(stop => stop());
  stops = [];
  document.body.replaceChildren();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  localStorage.clear();
  setLocale('en');
});
beforeEach(() => {
  Element.prototype.scrollIntoView = () => {};
  vi.stubGlobal('requestAnimationFrame', () => 1);
  vi.stubGlobal('cancelAnimationFrame', () => {});
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({ putImageData: vi.fn(), clearRect: vi.fn(), save: vi.fn(), restore: vi.fn(), measureText: () => ({ width: 120 }) } as unknown as CanvasRenderingContext2D);
});

type Mode = 'mounted in pseudo' | 'mounted in English, then switched';
type Initial = Partial<Omit<Session, 'preferences'>> & { preferences?: Partial<Preferences> };

/** Starts a session, mounts a surface on it, and leaves it in the pseudo language either way. */
async function open<Props extends Record<string, unknown>>(mode: Mode, surface: Component<Props>, props: (session: () => Session) => Omit<Props, 'session'>, initial: Initial = {}) {
  const language = mode === 'mounted in pseudo' ? 'pseudo' : 'system';
  useSession(createPreviewSession({ ...initial, preferences: { ...defaults.preferences, ...initial.preferences, language } }));
  let session = $state<Session>(structuredClone(defaults));
  stops.push(await subscribe(value => session = value));
  const current = () => session;
  const component = mount(surface, { target: document.body, props: { get session() { return session; }, ...props(current) } as unknown as Props });
  mounted.push(component);
  await settle();
  if (mode !== 'mounted in pseudo') {
    expect(locale()).toBe('en');
    await savePreferences({ ...current().preferences, language: 'pseudo' });
    await settle();
  }
  expect(locale()).toBe('pseudo');
}
async function settle() { await tick(); await tick(); flushSync(); }
const openSettings = (mode: Mode, initial?: Initial) => open(mode, Settings, session => ({ errors: createErrors(session) }), initial);
const expectNoLeaks = () => {
  expect(document.body.innerHTML).toContain('⟦');
  expect(leaks()).toEqual([]);
};
const tab = async (id: string) => { document.querySelector<HTMLElement>(`#tab-${id}`)!.click(); await settle(); };

it('the sweep notices a word that skipped t', () => {
  document.body.innerHTML = '<p>⟦Fine⟧ Leak</p><button aria-label="Close"></button><span lang="de">Deutsch</span><i title="#5856D6"></i>';
  expect(leaks()).toEqual(['text: ⟦Fine⟧ Leak', 'aria-label: Close']);
});

it('the pseudo catalog runs about 1.5 times the length of English', () => {
  const keys = Object.keys(pseudoCatalog());
  const ratio = keys.reduce((sum, key) => sum + pseudo(key).length, 0) / keys.reduce((sum, key) => sum + key.length, 0);
  expect(ratio).toBeGreaterThan(1.35);
  expect(ratio).toBeLessThan(1.7);
});

describe.each<Mode>(['mounted in pseudo', 'mounted in English, then switched'])('every visible string is translated when %s', mode => {
  it('Settings, General tab, with memory use and each update state', async () => {
    let emit: (status: UpdateStatus) => void = () => {};
    vi.mocked(watchUpdates).mockImplementation(async fn => { emit = fn; fn({ state: 'idle' }); return () => {}; });
    vi.mocked(getResourceUsage).mockResolvedValue({ memoryBytes: 118 * 1024 ** 2, cpuPercent: 0.4 });
    await openSettings(mode, { preferences: { autoFadeSeconds: 3 } });
    expect(document.querySelector('.usage')?.textContent).toContain('118 MB');
    expectNoLeaks();
    const statuses: UpdateStatus[] = [
      { state: 'disabled' }, { state: 'checking' }, { state: 'downloading', version: '0.2.0', percent: null }, { state: 'downloading', version: '0.2.0', percent: 42 },
      { state: 'ready', version: '0.2.0' }, { state: 'installing', version: '0.2.0' },
    ];
    for (const status of statuses) { emit(status); flushSync(); expectNoLeaks(); }
  });
  it('Settings, Colors tab, on a swatch and a sequence, with the palette list open and a bad hex code', async () => {
    await openSettings(mode);
    await tab('colors');
    expectNoLeaks();
    document.querySelector<HTMLElement>('[data-entry="0"]')!.click();
    await settle();
    expect(document.querySelector('.chip')).not.toBeNull();
    expectNoLeaks();
    document.querySelector<HTMLElement>('.trigger')!.click();
    await settle();
    expect(document.querySelector('[role="listbox"]')).not.toBeNull();
    expectNoLeaks();
    const hex = document.querySelector<HTMLInputElement>('.hex input')!;
    hex.value = 'nope';
    hex.dispatchEvent(new Event('input', { bubbles: true }));
    hex.dispatchEvent(new Event('blur'));
    await settle();
    expect(document.querySelector('.hex-error')).not.toBeNull();
    expectNoLeaks();
  });
  it('Settings, Keyboard shortcuts tab, with unavailable shortcuts, a recording row, and empty results', async () => {
    await openSettings(mode, { unavailableShortcuts: ['toggle', 'capture'] });
    await tab('keybindings');
    expect(document.querySelectorAll('.unavailable')).toHaveLength(2);
    expectNoLeaks();
    document.querySelector<HTMLElement>('.recorder')!.click();
    await settle();
    expect(document.querySelector('.recording')).not.toBeNull();
    expect(document.querySelector('.status p')?.textContent).toContain('⟦');
    expectNoLeaks();
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true, cancelable: true }));
    const search = document.querySelector<HTMLInputElement>('input[type="search"]')!;
    search.value = 'zzzzz';
    search.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    expect(document.querySelector('.empty-shortcuts')).not.toBeNull();
    expectNoLeaks();
    search.value = '';
    search.dispatchEvent(new Event('input', { bubbles: true }));
    document.querySelector<HTMLElement>('.customized-filter button')!.click();
    await settle();
    expect(document.querySelector('.empty-shortcuts')).not.toBeNull();
    expectNoLeaks();
  });
  it.each(['welcome', 'draw', 'hide', 'done'] as const)('Tutorial, %s step, with its unavailable-shortcut line and an error', async tutorial => {
    await open(mode, Tutorial, session => ({ errors: createErrors(session) }), { tutorial, unavailableShortcuts: ['toggle'], error: pseudo('Something went wrong') });
    expect(document.querySelector('.tutorial')).not.toBeNull();
    expect(document.querySelectorAll('.tutorial .error')).toHaveLength(2);
    expectNoLeaks();
  });
  it('Toolbar, with every tooltip, on each fade state', async () => {
    vi.useFakeTimers();
    for (const autoFadeSeconds of [0, 3] as const) {
      await open(mode, Toolbar, session => ({ errors: createErrors(session), host: 'page' as const, pinned: true, oncapture: () => {} }), {
        mode: 'draw', tutorial: null, historyByOverlay: autoFadeSeconds ? { 'overlay-0': { canUndo: true, canRedo: true } } : {}, preferences: { autoFadeSeconds, tutorialCompleted: true },
      });
      expectNoLeaks();
      let tips = 0;
      for (const button of document.querySelectorAll<HTMLElement>('.toolbar button')) {
        button.dispatchEvent(new Event('pointerenter'));
        await vi.advanceTimersByTimeAsync(200);
        await tick();
        const tip = document.querySelector('[role="tooltip"]');
        if (tip) { tips++; expect(tip.textContent).toContain('⟦'); }
        expect(leaks()).toEqual([]);
        button.dispatchEvent(new Event('pointerleave'));
      }
      expect(tips).toBeGreaterThan(12);
      await unmount(mounted.pop()!);
      stops.pop()!();
      document.body.replaceChildren();
    }
  });
  it('Toolbar, with an error showing', async () => {
    await open(mode, Toolbar, session => ({ errors: createErrors(session), host: 'page' as const }), { mode: 'draw', tutorial: null, error: pseudo('Something went wrong') });
    expect(document.querySelector('.error-banner')).not.toBeNull();
    expectNoLeaks();
  });
  it('Capture, choosing a region and then drawing on it', async () => {
    const getImage = vi.fn().mockResolvedValue({ width: 2560, height: 1600, data: new Uint8ClampedArray(0) });
    await open(mode, Capture, () => ({ getImage, copyImage: vi.fn() }), { capture: { id: 1, ready: true }, activeOverlay: 'capture', tutorial: null, preferences: { tutorialCompleted: true } });
    expectNoLeaks();
    const surface = document.querySelector<HTMLElement>('.selection-surface')!;
    surface.setPointerCapture = vi.fn();
    for (const [type, x, y] of [['pointerdown', 100, 100], ['pointerup', 600, 400]] as const) {
      surface.dispatchEvent(Object.assign(new MouseEvent(type, { bubbles: true, button: 0, clientX: x, clientY: y }), { pointerId: 1 }));
      flushSync();
    }
    await settle();
    expect(document.querySelector('button.copy')?.textContent).toContain('⟦');
    expectNoLeaks();
    document.querySelector<HTMLElement>('.tool')!.click();
    await settle();
    expect(document.querySelector('canvas[aria-label*="⟦"]')).not.toBeNull();
    expectNoLeaks();
  });
  it('Overlay', async () => {
    await open(mode, Overlay, () => ({ onerror: () => {} }), { mode: 'draw' });
    expect(document.querySelector('canvas')?.getAttribute('aria-label')).toContain('⟦');
    expectNoLeaks();
  });
});

describe('a language switch with no new error event', () => {
  const english = 'Could not capture the screen: busy';
  const translated = pseudo('Could not capture the screen: busy');
  it('shows the re-rendered error from the next snapshot and re-renders webview labels', async () => {
    let emit: (session: Session) => void = () => {};
    let session = $state<Session>(structuredClone(defaults));
    const start: Session = { ...structuredClone(defaults), error: english };
    const backend: SessionBackend = { ...createPreviewSession(), async subscribe(fn) { emit = fn; fn(start); return () => {}; } };
    useSession(backend);
    stops.push(await subscribe(value => session = value));
    mounted.push(mount(Settings, { target: document.body, props: { get session() { return session; }, errors: createErrors(() => session) } }));
    await settle();
    const error = () => document.querySelector('.status .error')?.textContent;
    const firstTab = () => document.querySelector('#tab-general')?.textContent;
    expect(error()).toBe(english);
    expect(firstTab()).toBe('General');
    // What Rust publishes after `activate`: the stored error re-rendered, with no `report`.
    emit({ ...start, locale: 'pseudo', error: translated });
    await settle();
    expect(error()).toBe(translated);
    expect(firstTab()).toBe(pseudo('General'));
    expect(t('General')).toBe(pseudo('General'));
    emit({ ...start, locale: 'en', error: english });
    await settle();
    expect(error()).toBe(english);
    expect(firstTab()).toBe('General');
  });
});
