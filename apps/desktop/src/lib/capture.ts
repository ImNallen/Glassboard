import { invoke } from '@tauri-apps/api/core';
import { native } from '@glassboard/ui/session';
import { previewCaptureImage, copyPreviewCapture } from '@glassboard/ui/capture-preview';

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
  return previewCaptureImage();
}

export async function copyCaptureImage(id: number, image: Promise<Blob>): Promise<void> {
  if (native) {
    const png = Array.from(new Uint8Array(await (await image).arrayBuffer()));
    return invoke<void>('copy_capture', { id, png });
  }
  await copyPreviewCapture(image);
}
