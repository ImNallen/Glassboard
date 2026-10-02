import preferenceDefaults from '../contract/preference-defaults.json';
import { cycleColor, CYCLE_COLORS } from './drawing';

const SWATCH_NAMES = ['Black', 'White', 'Green', 'Yellow', 'Red', 'Blue'];
/** The toolbar's solid color swatches, in toolbar order. Users can replace each color. */
export const DEFAULT_SWATCHES: readonly { color: string; name: string }[] = preferenceDefaults.swatches.map((color, slot) => ({ color, name: SWATCH_NAMES[slot] }));
export const defaultSwatches = () => DEFAULT_SWATCHES.map(swatch => swatch.color);

/** Quick picks for the color editor: the Glassboard palette, vivid hues, then neutrals and deep tones. */
export const SWATCH_PRESETS: readonly string[] = [
  '#f46b78', '#f2a65b', '#f2c85b', '#4dcaa0', '#4fc5d5', '#669df0', '#a184e8', '#e580b5',
  '#ff3b30', '#ff9500', '#ffcc00', '#34c759', '#00c7be', '#007aff', '#5856d6', '#ff2d55',
  '#000000', '#3a3f4b', '#6b7280', '#b8bec9', '#ffffff', '#8d5b3a', '#1f6f50', '#1e3a8a',
];

/** Rainbow and Shifting each use an ordered list of colors. */
export const SEQUENCE_LIMITS = { min: 2, max: 8 } as const;
export function sameColors(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((color, index) => sameColor(color, b[index]));
}

export function isHexColor(value: string): boolean { return /^#[0-9a-f]{6}$/i.test(value); }
export function sameColor(a: string, b: string): boolean { return a.toLowerCase() === b.toLowerCase(); }

/** The color in a swatch slot, falling back to the default for missing or invalid entries. */
export function swatchColor(swatches: readonly string[] | undefined, slot: number): string {
  const color = swatches?.[slot];
  return color && isHexColor(color) ? color : DEFAULT_SWATCHES[slot].color;
}

/** A default swatch keeps its name; a replaced one is named by its hex code. */
export function swatchName(swatches: readonly string[] | undefined, slot: number): string {
  const color = swatchColor(swatches, slot);
  return sameColor(color, DEFAULT_SWATCHES[slot].color) ? DEFAULT_SWATCHES[slot].name : color.toUpperCase();
}

/** Parse user input such as `#4DCAA0`, `4dcaa0`, or `#4ca`; undefined if it isn't a color. */
export function parseHexColor(value: string): string | undefined {
  const hex = value.trim().replace(/^#/, '').toLowerCase();
  if (/^[0-9a-f]{3}$/.test(hex)) return `#${[...hex].map(digit => digit + digit).join('')}`;
  if (/^[0-9a-f]{6}$/.test(hex)) return `#${hex}`;
}

export type Hsv = { h: number; s: number; v: number };

/** Hue in degrees, saturation and value from 0 to 1. */
export function hexToHsv(color: string): Hsv {
  const [r, g, b] = [1, 3, 5].map(index => parseInt(color.slice(index, index + 2), 16) / 255);
  const max = Math.max(r, g, b), delta = max - Math.min(r, g, b);
  let h = 0;
  if (delta) h = max === r ? ((g - b) / delta) % 6 : max === g ? (b - r) / delta + 2 : (r - g) / delta + 4;
  return { h: (h * 60 + 360) % 360, s: max ? delta / max : 0, v: max };
}

export function hsvToHex({ h, s, v }: Hsv): string {
  const channel = (n: number) => {
    const k = (n + h / 60) % 6;
    return Math.round((v - v * s * Math.max(0, Math.min(k, 4 - k, 1))) * 255).toString(16).padStart(2, '0');
  };
  return `#${channel(5)}${channel(3)}${channel(1)}`;
}

/** A swatch showing a rainbow's colors all the way around. */
export function rainbowPreview(colors: readonly string[] = CYCLE_COLORS): string { return `conic-gradient(${[...colors, colors[0]].join(', ')})`; }
/** A swatch showing three colors spread across the Shifting sequence, starting at `index`. */
export function shiftingPreview(colors: readonly string[] = CYCLE_COLORS, index = 0): string {
  const [a, b, c] = [0, 1, 2].map(step => cycleColor(index + Math.round(step * colors.length / 3), colors));
  return `conic-gradient(${a} 0deg 120deg, ${b} 120deg 240deg, ${c} 240deg 360deg)`;
}
/** A strip blending through `colors` from left to right. */
export function gradientStrip(colors: readonly string[]): string { return `linear-gradient(90deg, ${colors.join(', ')})`; }
/** A strip of equal hard-edged bands, one per color. */
export function bandStrip(colors: readonly string[]): string {
  return `linear-gradient(90deg, ${colors.map((color, i) => `${color} ${i / colors.length * 100}% ${(i + 1) / colors.length * 100}%`).join(', ')})`;
}
