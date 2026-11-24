export interface AccentPreset {
  id: string;
  name: string;
  hex: string;
}

export const ACCENT_PRESETS: readonly AccentPreset[] = [
  { id: 'cyan', name: 'Keynear cyan', hex: '#20BCDA' },
  { id: 'violet', name: 'Violet', hex: '#7C5CFC' },
  { id: 'green', name: 'Green', hex: '#22A06B' },
  { id: 'amber', name: 'Amber', hex: '#E8A33D' },
  { id: 'rose', name: 'Brand rose', hex: '#ED012B' },
];

export const DEFAULT_ACCENT = ACCENT_PRESETS[0].hex;

const OVERRIDDEN_PROPERTIES = [
  '--kn-accent-fill',
  '--kn-loading',
  '--kn-sys-accent',
  '--kn-loading-ink',
  '--kn-brand-ink',
  '--kn-focus-ring',
  '--kn-sys-accent-ink',
  '--kn-loading-tonal',
  '--kn-sys-accent-tonal',
  '--kn-on-loading-tonal',
  '--kn-sys-on-accent-tonal',
  '--kn-grad-accent',
  '--kn-grad-accent-hover',
  '--kn-brand-depth-accent',
  '--kn-brand-depth-accent-pressed',
  '--kn-brand-aura',
  '--kn-map-contour',
  '--kn-map-contour-far',
] as const;

/** Applies an accent override on the document root; the default clears every override. */
export function applyAccent(hex: string, dark: boolean) {
  const root = document.documentElement;
  if (hex.toLowerCase() === DEFAULT_ACCENT.toLowerCase()) {
    OVERRIDDEN_PROPERTIES.forEach((property) => root.style.removeProperty(property));
    return;
  }

  const ink = dark ? hex : `color-mix(in oklab, ${hex} 72%, #071A21)`;
  const tonal = `color-mix(in oklab, ${hex} 14%, ${dark ? '#132C35' : '#FFFFFF'})`;
  const shadow = `0 6px 18px -6px color-mix(in oklab, ${hex} 52%, transparent), 0 1px 2px rgba(5, 31, 39, 0.08)`;
  const values: Record<string, string> = {
    '--kn-accent-fill': hex,
    '--kn-loading': hex,
    '--kn-sys-accent': hex,
    '--kn-loading-ink': ink,
    '--kn-brand-ink': ink,
    '--kn-focus-ring': ink,
    '--kn-sys-accent-ink': ink,
    '--kn-loading-tonal': tonal,
    '--kn-sys-accent-tonal': tonal,
    '--kn-on-loading-tonal': ink,
    '--kn-sys-on-accent-tonal': ink,
    '--kn-grad-accent':
      `radial-gradient(95% 110% at 18% -10%, color-mix(in oklab, #FFFFFF 58%, ${hex}) 0%, transparent 56%), ` +
      `radial-gradient(85% 110% at 88% 115%, color-mix(in oklab, ${hex} 42%, #60B2F0) 0%, transparent 60%)`,
    '--kn-grad-accent-hover':
      `radial-gradient(100% 115% at 20% -10%, color-mix(in oklab, #FFFFFF 72%, ${hex}) 0%, transparent 58%), ` +
      `radial-gradient(85% 110% at 88% 115%, color-mix(in oklab, ${hex} 45%, #72C6F8) 0%, transparent 62%)`,
    '--kn-brand-depth-accent': shadow,
    '--kn-brand-depth-accent-pressed': `0 2px 6px -2px color-mix(in oklab, ${hex} 35%, transparent)`,
    '--kn-brand-aura':
      `radial-gradient(circle, color-mix(in oklab, ${hex} 24%, transparent) 0%, ` +
      `color-mix(in oklab, ${hex} 7%, transparent) 46%, transparent 72%)`,
    '--kn-map-contour': `color-mix(in oklab, ${hex} 22%, transparent)`,
    '--kn-map-contour-far': `color-mix(in oklab, ${hex} 16%, transparent)`,
  };
  Object.entries(values).forEach(([property, value]) => root.style.setProperty(property, value));
}
