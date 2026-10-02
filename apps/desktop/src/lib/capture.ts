import { invoke } from '@tauri-apps/api/core';
import type { CaptureBackend } from '@glassboard/ui/capture';

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

export const nativeCapture: CaptureBackend = {
  async getImage(id) {
    const frame = await invoke<ArrayBuffer | number[]>('get_capture_image', { id });
    // Tauri normally returns an ArrayBuffer. Its postMessage fallback uses a byte array.
    return decodeCaptureFrame(frame instanceof ArrayBuffer ? frame : new Uint8Array(frame).buffer);
  },
  async copyImage(id, image) {
    const png = await (await image).arrayBuffer();
    return invoke<void>('copy_capture', png, { headers: { 'x-glassboard-capture-id': String(id) } });
  },
  /** Plain captures stay native: only the selected pixel bounds cross the bridge. */
  copyRegion: (id, region) => invoke<void>('copy_capture_region', { id, region }),
};
