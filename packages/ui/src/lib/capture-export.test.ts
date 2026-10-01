// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { annotatedCapture } from './capture';
import { render } from './drawing';

vi.mock('./drawing', () => ({ render: vi.fn() }));
afterEach(() => vi.restoreAllMocks());

function setup() {
  vi.mocked(render).mockClear();
  const drawImage = vi.fn();
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({ drawImage } as unknown as CanvasRenderingContext2D);
  vi.spyOn(HTMLCanvasElement.prototype, 'toBlob').mockImplementation(callback => callback(new Blob(['png'], { type: 'image/png' })));
  const image = document.createElement('canvas'); image.width = 1000; image.height = 600;
  const create = vi.spyOn(document, 'createElement');
  return { image, create, drawImage };
}

it('crops and draws annotations on one canvas while preserving the screenshot pixels', async () => {
  const { image, create, drawImage } = setup();
  const points = [{ x: 25, y: 35 }, { x: 70, y: 55 }];
  const shape = { id: 'arrow', tool: 'arrow' as const, points, width: 4, color: '#669df0', expiresAt: 1000 };
  const blob = await annotatedCapture(image, { x: 10.25, y: 20.75, width: 40.5, height: 30.5 }, [shape], { width: 500, height: 300 });
  expect(blob.type).toBe('image/png');
  expect(create).toHaveBeenCalledTimes(1);
  expect(drawImage).toHaveBeenCalledExactlyOnceWith(image, 20, 41, 82, 62, 0, 0, 82, 62);
  expect(render).toHaveBeenCalledWith(expect.anything(), [expect.objectContaining({
    points: [{ x: 15, y: 14.5 }, { x: 60, y: 34.5 }], expiresAt: undefined, fadeSeconds: 0,
  })], null, 41, 31, 2, expect.any(Number), false);
  expect(shape.points).toBe(points);
  expect(shape.expiresAt).toBe(1000);
});

it('skips annotation rendering for browser copies of a plain region', async () => {
  const { image, create, drawImage } = setup();
  await annotatedCapture(image, { x: 0, y: 0, width: 100, height: 50 }, [], { width: 500, height: 300 });
  expect(create).toHaveBeenCalledTimes(1);
  expect(drawImage).toHaveBeenCalledTimes(1);
  expect(render).not.toHaveBeenCalled();
});

it('rejects an empty crop before allocating an export canvas', async () => {
  const { image, create } = setup();
  await expect(annotatedCapture(image, { x: 0, y: 0, width: 0, height: 50 }, [], { width: 500, height: 300 })).rejects.toThrow('Select a region');
  expect(create).not.toHaveBeenCalled();
});
