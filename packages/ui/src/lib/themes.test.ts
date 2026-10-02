import { describe, expect, it } from 'vitest';
import { COLOR_THEMES, matchTheme, themeColors } from './themes';
import { DEFAULT_SWATCHES, defaultSwatches, isHexColor, SEQUENCE_LIMITS } from './swatches';
import { CYCLE_COLORS } from './drawing';

describe('color themes', () => {
  it('starts with the default Glassboard colors', () => {
    expect(COLOR_THEMES[0].colors).toEqual(CYCLE_COLORS);
    expect(COLOR_THEMES[0].swatches).toEqual(DEFAULT_SWATCHES.slice(2).map(swatch => swatch.color));
  });
  it.each(COLOR_THEMES.map(theme => [theme.name, theme] as const))('%s is a valid theme', (_, theme) => {
    expect(theme.colors.length).toBeGreaterThanOrEqual(SEQUENCE_LIMITS.min);
    expect(theme.colors.length).toBeLessThanOrEqual(SEQUENCE_LIMITS.max);
    expect(theme.swatches).toHaveLength(4);
    for (const color of [...theme.colors, ...theme.swatches]) expect(isHexColor(color) && color === color.toLowerCase()).toBe(true);
  });
  it('has unique names', () => {
    expect(new Set(COLOR_THEMES.map(theme => theme.name)).size).toBe(COLOR_THEMES.length);
  });
  it('applies a theme over the swatches, keeping Black and White', () => {
    const neon = COLOR_THEMES[2];
    const colors = themeColors(neon, ['#111111', '#222222', '#333333', '#444444', '#555555', '#666666']);
    expect(colors).toEqual({ swatches: ['#111111', '#222222', ...neon.swatches], rainbowColors: neon.colors, cycleColors: neon.colors });
    expect(matchTheme(colors)).toBe(neon);
  });
  it('matches a theme only when every themed color is in use', () => {
    const defaults = { swatches: defaultSwatches(), rainbowColors: [...COLOR_THEMES[0].colors], cycleColors: [...COLOR_THEMES[0].colors] };
    expect(matchTheme(defaults)).toBe(COLOR_THEMES[0]);
    expect(matchTheme({ ...defaults, swatches: undefined })).toBe(COLOR_THEMES[0]);
    expect(matchTheme({ ...defaults, swatches: ['#123456', '#654321', ...defaults.swatches.slice(2).map(color => color.toUpperCase())] })).toBe(COLOR_THEMES[0]);
    expect(matchTheme({ ...defaults, cycleColors: COLOR_THEMES[1].colors })).toBeUndefined();
    expect(matchTheme({ ...defaults, rainbowColors: COLOR_THEMES[1].colors })).toBeUndefined();
    expect(matchTheme({ ...defaults, swatches: defaults.swatches.map((color, slot) => slot === 5 ? '#123456' : color) })).toBeUndefined();
  });
});
