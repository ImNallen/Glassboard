import { render, type Point, type Shape } from './drawing';
import { isEditableTarget } from './selection';
import { commandFor, keybinding, sameShortcut, type Keybindings } from './shortcuts';

export type CaptureRegion = { x: number; y: number; width: number; height: number };
/** Where the capture editor gets its frozen screen and sends the copy. Only hosts that crop natively provide `copyRegion`. */
export type CaptureBackend = { getImage: (id: number) => Promise<ImageData | HTMLImageElement | HTMLCanvasElement>; copyImage: (id: number, image: Promise<Blob>) => Promise<void>; copyRegion?: (id: number, region: CaptureRegion) => Promise<void> };

/** Normalize either drag direction, keeping a selection inside its display. */
export function captureRegion(from: Point, to: Point, width: number, height: number): CaptureRegion {
  const clamp = (value: number, limit: number) => Math.max(0, Math.min(value, limit));
  const x = clamp(Math.min(from.x, to.x), width), y = clamp(Math.min(from.y, to.y), height);
  return { x, y, width: clamp(Math.max(from.x, to.x), width) - x, height: clamp(Math.max(from.y, to.y), height) - y };
}

/** Include edge pixels and retain the screenshot's actual density, rather than the webview DPR. */
export function capturePixels(region: CaptureRegion, viewport: { width: number; height: number }, image: { width: number; height: number }): CaptureRegion {
  const scaleX = image.width / viewport.width, scaleY = image.height / viewport.height;
  const x = Math.max(0, Math.floor(region.x * scaleX)), y = Math.max(0, Math.floor(region.y * scaleY));
  const right = Math.min(image.width, Math.ceil((region.x + region.width) * scaleX));
  const bottom = Math.min(image.height, Math.ceil((region.y + region.height) * scaleY));
  return { x, y, width: Math.max(0, right - x), height: Math.max(0, bottom - y) };
}

/** Compose only screenshot pixels and drawings; editor chrome never enters the export. */
export function annotatedCapture(image: HTMLImageElement | HTMLCanvasElement, region: CaptureRegion, shapes: Shape[], viewport = { width: window.innerWidth, height: window.innerHeight }): Promise<Blob> {
  const size = image instanceof HTMLImageElement ? { width: image.naturalWidth, height: image.naturalHeight } : image;
  const source = capturePixels(region, viewport, size);
  if (!source.width || !source.height) return Promise.reject(new Error('Select a region to copy.'));
  const scaleX = size.width / viewport.width, scaleY = size.height / viewport.height;
  const output = document.createElement('canvas');
  output.width = source.width; output.height = source.height;
  const ctx = output.getContext('2d');
  if (!ctx) return Promise.reject(new Error('Could not create the image. Try again.'));
  ctx.drawImage(image, source.x, source.y, source.width, source.height, 0, 0, source.width, source.height);
  const translated = shapes.map(shape => ({ ...shape,
    points: shape.points.map(point => ({ x: point.x - source.x / scaleX, y: (point.y * scaleY - source.y) / scaleX })),
    fadeSeconds: 0 as const, expiresAt: undefined,
  }));
  if (translated.length) render(ctx, translated, null, source.width / scaleX, source.height / scaleX, scaleX, Date.now(), false);
  return new Promise((resolve, reject) => output.toBlob(blob => {
    output.width = 0; output.height = 0;
    if (blob) resolve(blob);
    else reject(new Error('Could not encode the image. Try again.'));
  }, 'image/png'));
}

export function captureKeydown(event: KeyboardEvent, { copy, cancel, keybindings }: { copy: () => void; cancel: () => void; keybindings?: Keybindings }) {
  if (event.defaultPrevented) return;
  // Copy remains normal text copy while a text editor has focus.
  if (isEditableTarget(event.target)) return;
  const command = commandFor(event, keybindings);
  if (command === 'copy') {
    event.preventDefault();
    if (!event.repeat) copy();
  } else if (command === 'hide') {
    event.preventDefault();
    if (!event.repeat) cancel();
  }
}

/**
 * Whether a clipboard `copy` event (from Cmd/Ctrl+C or Edit › Copy) should copy the capture.
 * Only while Copy is still bound to the standard shortcut, so rebinding or unbinding it takes effect.
 */
export function copiesOnClipboardEvent(keybindings?: Keybindings): boolean {
  return sameShortcut(keybinding(keybindings, 'copy'), 'CommandOrControl+KeyC');
}
