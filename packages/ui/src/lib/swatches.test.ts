import { describe, expect, it } from 'vitest';
import { defaultSwatches, hexToHsv, hsvToHex, parseHexColor, swatchColor, swatchName, SWATCH_PRESETS } from './swatches';

describe('swatches', () => {
  it('keeps the toolbar palette as the default', () => {
    expect(defaultSwatches()).toEqual(['#000000', '#ffffff', '#4dcaa0', '#f2c85b', '#f46b78', '#669df0']);
    expect(new Set(SWATCH_PRESETS).size).toBe(SWATCH_PRESETS.length);
  });
  it('falls back to defaults for missing or invalid colors', () => {
    expect(swatchColor(undefined, 4)).toBe('#f46b78');
    expect(swatchColor(['#000000', 'red'], 1)).toBe('#ffffff');
    expect(swatchColor(['#123ABC'], 0)).toBe('#123ABC');
  });
  it('names default swatches by color and replaced ones by hex code', () => {
    const swatches = defaultSwatches();
    expect(swatchName(swatches, 4)).toBe('Red');
    swatches[4] = '#8b5cf6';
    expect(swatchName(swatches, 4)).toBe('#8B5CF6');
    expect(swatchName(['#000000', '#FFFFFF'], 1)).toBe('White');
  });
  it('parses typed hex codes', () => {
    expect(parseHexColor('#4DCAA0')).toBe('#4dcaa0');
    expect(parseHexColor(' 4dcaa0 ')).toBe('#4dcaa0');
    expect(parseHexColor('#4ca')).toBe('#44ccaa');
    for (const value of ['', '#', '#12345', '#gggggg', 'red', '#1234567']) expect(parseHexColor(value)).toBeUndefined();
  });
  it('converts between hex and HSV without drift', () => {
    expect(hexToHsv('#ff0000')).toEqual({ h: 0, s: 1, v: 1 });
    expect(hexToHsv('#000000')).toEqual({ h: 0, s: 0, v: 0 });
    expect(hsvToHex({ h: 120, s: 1, v: 1 })).toBe('#00ff00');
    for (const color of [...SWATCH_PRESETS, ...defaultSwatches()]) expect(hsvToHex(hexToHsv(color))).toBe(color);
  });
});
