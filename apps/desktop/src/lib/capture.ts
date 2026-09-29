import { invoke } from '@tauri-apps/api/core';
import { native } from '@glassboard/ui/session';

/** Read a native binary frame without PNG decompression or reducing pixel density. */
export function decodeCaptureFrame(frame: ArrayBuffer): ImageData {
  if (frame.byteLength < 8) throw new Error('The screenshot frame is incomplete.');
  const header = new DataView(frame);
  const width = header.getUint32(0, true), height = header.getUint32(4, true);
  if (!width || !height || width * height * 4 !== frame.byteLength - 8) {
    throw new Error('The screenshot frame has invalid dimensions.');
  }
  return new ImageData(new Uint8ClampedArray(frame, 8), width, height);
}

export async function getCaptureImage(id: number): Promise<ImageData | HTMLImageElement> {
  if (native) {
    const frame = await invoke<ArrayBuffer | number[]>('get_capture_image', { id });
    // Tauri normally returns an ArrayBuffer. Its postMessage fallback uses a byte array.
    return decodeCaptureFrame(frame instanceof ArrayBuffer ? frame : new Uint8Array(frame).buffer);
  }
  // Browser previews cannot silently screenshot other apps. A frozen sample exercises
  // exactly the same region selection, drawing, composition, and image clipboard path.
  const width = window.innerWidth, height = window.innerHeight;
  const escape = (value: string) => value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
  const lines = ['function makeYourPoint() {', '  const capture = screen.selectRegion();', '  capture.annotate("This is the part I mean");', '  clipboard.copy(capture);', '}', '', '// Capture. Explain. Paste into your conversation.'];
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width * 2}" height="${height * 2}" viewBox="0 0 ${width} ${height}"><rect width="100%" height="100%" fill="#edf2ef"/><rect x="40" y="80" width="${Math.max(100, width - 80)}" height="${Math.max(100, height - 180)}" rx="16" fill="#202b30"/><text x="70" y="124" fill="#a3e9d1" font-family="sans-serif" font-size="16">Glassboard · Screenshot preview</text>${lines.map((line, i) => `<text x="70" y="${180 + i * 32}" fill="${i === 6 ? '#8ea69b' : '#e3ede7'}" font-family="monospace" font-size="16">${escape(line)}</text>`).join('')}</svg>`;
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error('Could not load the screenshot preview.'));
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  });
}

export async function copyCaptureImage(id: number, image: Promise<Blob>): Promise<void> {
  if (native) {
    const png = Array.from(new Uint8Array(await (await image).arrayBuffer()));
    return invoke<void>('copy_capture', { id, png });
  }
  if (!navigator.clipboard?.write || typeof ClipboardItem === 'undefined') {
    throw new Error('Image copy is unavailable in this browser. Use the desktop app to copy captures.');
  }
  // Supply the promise during the user gesture, including on Safari.
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': image })]);
}
