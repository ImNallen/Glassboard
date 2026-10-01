import { CYCLE_COLORS } from './drawing';

/**
 * A color theme for the toolbar. Applying one sets Rainbow and Shifting to `colors`
 * and the four swatches after Black and White (keys 5–8) to `swatches`.
 * To add a theme, append an entry: 2–8 `colors` and exactly 4 `swatches`, all `#rrggbb`.
 */
export type ColorTheme = { name: string; colors: readonly string[]; swatches: readonly [string, string, string, string] };

export const COLOR_THEMES: readonly ColorTheme[] = [
  { name: 'Glassboard', colors: CYCLE_COLORS, swatches: ['#4dcaa0', '#f2c85b', '#f46b78', '#669df0'] },
  { name: 'Spectrum', colors: ['#ff3b30', '#ff9500', '#ffcc00', '#34c759', '#007aff', '#5856d6', '#af52de'], swatches: ['#34c759', '#ffcc00', '#ff3b30', '#007aff'] },
  { name: 'Neon', colors: ['#ff2bd6', '#ffe600', '#00ff9c', '#00e5ff', '#7c4dff'], swatches: ['#00ff9c', '#ffe600', '#ff2bd6', '#00e5ff'] },
  { name: 'Sunset', colors: ['#ff5e62', '#ff9966', '#ffc371', '#f7797d', '#c06c84', '#6c5b7b'], swatches: ['#ffc371', '#ff9966', '#ff5e62', '#6c5b7b'] },
  { name: 'Ocean', colors: ['#0b3d91', '#1e6fd9', '#22a6f2', '#38d6c4', '#a0f0e0'], swatches: ['#38d6c4', '#a0f0e0', '#22a6f2', '#0b3d91'] },
  { name: 'Pastel', colors: ['#ffb3ba', '#ffdfba', '#ffffba', '#baffc9', '#bae1ff', '#d5baff'], swatches: ['#baffc9', '#ffffba', '#ffb3ba', '#bae1ff'] },
];

/** The first two swatches, Black and White, are not part of a theme. */
export const THEMED_SWATCHES_START = 2;
