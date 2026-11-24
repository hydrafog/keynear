export interface AccentPreset {
  id: string;
  name: string;
  hex: string;
}

export const ACCENT_PRESETS: readonly AccentPreset[] = [
  { id: 'cyan', name: 'Keynear cyan', hex: '#4ABBD5' },
  { id: 'violet', name: 'Violet', hex: '#C594E9' },
  { id: 'green', name: 'Green', hex: '#6AC380' },
  { id: 'amber', name: 'Amber', hex: '#E79750' },
  { id: 'rose', name: 'Rose', hex: '#F3898A' },
];

export const DEFAULT_ACCENT = ACCENT_PRESETS[0].hex;

const LIGHTNESS_THRESHOLD = 0.68;

function srgbToLinear(channel: number): number {
  const c = channel / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

/** Computes perceptual OKLCH lightness L in [0, 1] for a 6-digit hex color. */
export function getOklchLightness(hex: string): number {
  const clean = hex.replace('#', '');
  const r = srgbToLinear(parseInt(clean.slice(0, 2), 16) || 0);
  const g = srgbToLinear(parseInt(clean.slice(2, 4), 16) || 0);
  const b = srgbToLinear(parseInt(clean.slice(4, 6), 16) || 0);

  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);

  return 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s;
}

/** Returns `#071A21` when the accent's OKLCH lightness exceeds 0.68, otherwise `#FFFFFF`. */
export function getOnAccentColor(hex: string): string {
  return getOklchLightness(hex) > LIGHTNESS_THRESHOLD ? '#071A21' : '#FFFFFF';
}

/** Applies the chosen accent color and derived contrast tokens uniformly across all presets and custom picks. */
export function applyAccent(hex: string, _dark: boolean) {
  const root = document.documentElement;
  const onAccent = getOnAccentColor(hex);
  root.style.setProperty('--kn-accent-fill', hex);
  root.style.setProperty('--kn-on-accent', onAccent);
  root.style.setProperty('--kn-sys-on-accent', onAccent);

  const metaTheme = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');
  if (metaTheme) {
    metaTheme.content = hex;
  }
}
