// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { decodeCaptureFrame, getCaptureImage, copyCaptureImage, copyCaptureRegion } from './capture';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@glassboard/ui/session', () => ({ native: true }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

afterEach(() => { vi.unstubAllGlobals(); vi.clearAllMocks(); });

it('copies a plain screenshot using only capture identity and source pixel bounds', async () => {
  const region = { x: 20, y: 41, width: 82, height: 62 };
  await copyCaptureRegion(42, region);
  expect(invoke).toHaveBeenCalledExactlyOnceWith('copy_capture_region', { id: 42, region });
});

it('copies PNG bytes as a binary request with capture identity outside the body', async () => {
  const png = new Uint8Array([137, 80, 78, 71, 0, 255]).buffer;
  const image = { arrayBuffer: async () => png } as Blob;
  await copyCaptureImage(42, Promise.resolve(image));
  expect(invoke).toHaveBeenCalledExactlyOnceWith('copy_capture', png, { headers: { 'x-glassboard-capture-id': '42' } });
});

it('propagates a clipboard failure so the editor can keep the capture open', async () => {
  vi.mocked(invoke).mockRejectedValueOnce(new Error('clipboard unavailable'));
  const image = { arrayBuffer: async () => new ArrayBuffer(8) } as Blob;
  await expect(copyCaptureImage(42, Promise.resolve(image))).rejects.toThrow('clipboard unavailable');
});

it('reads little-endian dimensions and shares the full-resolution RGBA bytes without a copy', () => {
  vi.stubGlobal('ImageData', class {
    constructor(public data: Uint8ClampedArray, public width: number, public height: number) {}
  });
  const frame = new ArrayBuffer(8 + 2 * 3 * 4);
  const header = new DataView(frame);
  header.setUint32(0, 2, true); header.setUint32(4, 3, true);
  const pixels = new Uint8Array(frame, 8);
  pixels.set([10, 20, 30, 255, 40, 50, 60, 128]);
  const image = decodeCaptureFrame(frame);
  expect(image.width).toBe(2); expect(image.height).toBe(3);
  expect(image.data.buffer).toBe(frame);
  expect([...image.data.slice(0, 8)]).toEqual([10, 20, 30, 255, 40, 50, 60, 128]);
});

it('rejects truncated headers, empty dimensions, and missing or extra pixel bytes', () => {
  expect(() => decodeCaptureFrame(new ArrayBuffer(7))).toThrow('incomplete');
  for (const [width, height, length] of [[0, 1, 8], [1, 0, 8], [2, 3, 31], [2, 3, 33], [0xffffffff, 0xffffffff, 8]]) {
    const frame = new ArrayBuffer(length);
    const header = new DataView(frame);
    header.setUint32(0, width, true); header.setUint32(4, height, true);
    expect(() => decodeCaptureFrame(frame)).toThrow('invalid dimensions');
  }
});

it.each([false, true])('loads native frames with the IPC byte-array fallback %s', async fallback => {
  vi.stubGlobal('ImageData', class {
    constructor(public data: Uint8ClampedArray, public width: number, public height: number) {}
  });
  const frame = new Uint8Array([1, 0, 0, 0, 1, 0, 0, 0, 11, 22, 33, 255]);
  vi.mocked(invoke).mockResolvedValueOnce(fallback ? [...frame] : frame.buffer);
  const image = await getCaptureImage(42) as ImageData;
  expect(invoke).toHaveBeenLastCalledWith('get_capture_image', { id: 42 });
  expect(image.width).toBe(1); expect(image.height).toBe(1);
  expect([...image.data]).toEqual([11, 22, 33, 255]);
});
