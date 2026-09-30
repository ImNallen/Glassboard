import { render, type Point, type Shape } from './drawing';

export type CaptureRegion = { x: number; y: number; width: number; height: number };

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
  const annotations = document.createElement('canvas');
  annotations.width = source.width; annotations.height = source.height;
  const ctx = output.getContext('2d'), drawing = annotations.getContext('2d');
  if (!ctx || !drawing) return Promise.reject(new Error('Could not create the image. Try again.'));
  ctx.drawImage(image, source.x, source.y, source.width, source.height, 0, 0, source.width, source.height);
  const translated = shapes.map(shape => ({ ...shape,
    points: shape.points.map(point => ({ x: point.x - source.x / scaleX, y: (point.y * scaleY - source.y) / scaleX })),
    fadeSeconds: 0 as const, expiresAt: undefined,
  }));
  render(drawing, translated, null, source.width / scaleX, source.height / scaleX, scaleX);
  ctx.drawImage(annotations, 0, 0);
  return new Promise((resolve, reject) => output.toBlob(blob => blob ? resolve(blob) : reject(new Error('Could not encode the image. Try again.')), 'image/png'));
}

export function captureKeydown(event: KeyboardEvent, { copy, cancel }: { copy: () => void; cancel: () => void }) {
  if (event.defaultPrevented) return;
  const target = event.target instanceof Element ? event.target : document.activeElement;
  const editable = target?.closest('input, textarea, select, [contenteditable="true"]');
  const key = event.key.toLowerCase();
  // Copy remains normal text copy while a text editor has focus.
  if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && key === 'c' && !editable) {
    event.preventDefault();
    if (!event.repeat) copy();
  } else if (key === 'escape' && !editable) {
    event.preventDefault(); cancel();
  }
}
