// @ts-check

/** @typedef {'macos' | 'windows' | 'linux' | null} DesktopPlatform */
/** @typedef {{ userAgent?: string, platform?: string, maxTouchPoints?: number, userAgentData?: { platform?: string, mobile?: boolean } }} PlatformHints */

/** @type {{ platform: Exclude<DesktopPlatform, null>, hint: RegExp, identity: RegExp }[]} */
const platforms = [
  { platform: 'macos', hint: /^macOS$/i, identity: /Macintosh|MacIntel|MacPPC|Mac68K/i },
  { platform: 'windows', hint: /^Windows$/i, identity: /Windows|Win32|Win64/i },
  { platform: 'linux', hint: /^Linux$/i, identity: /Linux/i },
];

/** @param {PlatformHints} hints @returns {DesktopPlatform} */
export function detectDesktopPlatform(hints) {
  const identity = `${hints.userAgent ?? ''} ${hints.platform ?? ''}`;
  if (hints.userAgentData?.mobile || /Android|iPhone|iPad|iPod|Mobile|CrOS|ChromeOS/i.test(identity)) return null;
  if (/Macintosh|MacIntel|MacPPC|Mac68K/i.test(identity) && (hints.maxTouchPoints ?? 0) > 1) return null;

  const hint = hints.userAgentData?.platform?.trim();
  const match = platforms.find(entry => hint ? entry.hint.test(hint) : entry.identity.test(identity));
  return match?.platform ?? null;
}

/** @param {Document} document @param {PlatformHints} hints */
export function enhancePlatformDownloads(document, hints) {
  const platform = detectDesktopPlatform(hints);
  if (!platform) return;

  const list = document.getElementById('downloads');
  const toggle = document.querySelector('.download-toggle');
  if (!list || !(toggle instanceof HTMLButtonElement)) return;
  const cards = Array.from(list.querySelectorAll('div.download'));
  if (!cards.every(card => card instanceof HTMLElement)) return;
  const recommended = cards.find(card => card.dataset.platform === platform);
  const alternatives = cards.filter(card => card !== recommended);
  const firstAlternative = alternatives[0]?.querySelector('a');
  if (!recommended || !alternatives.length || !firstAlternative) return;

  let expanded = false;
  toggle.addEventListener('click', () => {
    expanded = !expanded;
    for (const card of alternatives) card.hidden = !expanded;
    toggle.setAttribute('aria-expanded', String(expanded));
    toggle.textContent = expanded ? 'Show recommended download' : 'Show all downloads';
    if (expanded) firstAlternative.focus();
    else toggle.focus();
  });

  list.prepend(recommended);
  for (const card of alternatives) card.hidden = true;
  toggle.hidden = false;
}
