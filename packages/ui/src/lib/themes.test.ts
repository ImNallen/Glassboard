import { describe, expect, it } from 'vitest';
import { COLOR_THEMES } from './themes';
import { DEFAULT_SWATCHES, isHexColor, SEQUENCE_LIMITS } from './swatches';
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
});
