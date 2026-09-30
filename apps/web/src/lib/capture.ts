import { toCanvas } from 'html-to-image';

/** Freeze the visible landing page, excluding Glassboard's editor and controls. */
export async function capturePage(): Promise<HTMLCanvasElement> {
  await document.fonts.ready;
  const scale = window.devicePixelRatio || 1;
  const viewport = { width: window.innerWidth, height: window.innerHeight, x: window.scrollX, y: window.scrollY };
  const background = getComputedStyle(document.documentElement).backgroundColor;
  // Keep the root's font metrics and scrollbar gutter in the rendered document.
  // Rendering only body gives rem units a different root and omits the gutter.
  const root = document.documentElement;
  // html-to-image rounds down font-size. Copy the complete font shorthand instead
  // so balanced headings and text near a wrap boundary keep their original lines.
  const includeStyleProperties = [...getComputedStyle(root)].filter(property => property !== 'font-size');
  includeStyleProperties.push('font');
  const page = await toCanvas(root, {
    width: viewport.width,
    height: Math.max(root.scrollHeight, viewport.height),
    pixelRatio: scale,
    backgroundColor: background,
    includeStyleProperties,
    filter: element => element.tagName !== 'HEAD' && !element.classList?.contains('glassboard-layer'),
  });
  // Cloning is asynchronous. Do not display a frame whose layout was rendered
  // across two different viewports or scroll positions.
  if (window.innerWidth !== viewport.width || window.innerHeight !== viewport.height
    || window.scrollX !== viewport.x || window.scrollY !== viewport.y
    || (window.devicePixelRatio || 1) !== scale) {
    throw new Error('The page moved or resized during capture. Try again.');
  }
  const image = document.createElement('canvas');
  image.width = Math.round(viewport.width * scale);
  image.height = Math.round(viewport.height * scale);
  const context = image.getContext('2d');
  if (!context) throw new Error('Could not capture the page. Try again.');
  context.fillStyle = background;
  context.fillRect(0, 0, image.width, image.height);
  context.drawImage(page, -viewport.x * scale, -viewport.y * scale);
  return image;
}
