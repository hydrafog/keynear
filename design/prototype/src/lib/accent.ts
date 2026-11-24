export interface AccentPreset {
  id: string;
  name: string;
  hex: string;
}

// Every preset holds OKLCH L=74.4% and the same in-gamut chroma C=0.1296 and changes only hue.
// C=0.145 at L=74.4%, H=212 (the CSS fill) is outside sRGB and clips to C=0.1296.
export const ACCENT_PRESETS: readonly AccentPreset[] = [
  { id: 'cyan', name: 'Keynear cyan', hex: '#00C1DB' },
  { id: 'violet', name: 'Violet', hex: '#C594E9' },
  { id: 'green', name: 'Green', hex: '#6AC380' },
  { id: 'amber', name: 'Amber', hex: '#E79750' },
  { id: 'rose', name: 'Rose', hex: '#F3898A' },
];

export const DEFAULT_ACCENT = ACCENT_PRESETS[0].hex;

const OVERRIDDEN_PROPERTIES = [
  '--kn-accent-fill',
  '--kn-loading',
  '--kn-loading-hover',
  '--kn-loading-ink',
  '--kn-loading-tonal',
  '--kn-on-loading-tonal',
  '--kn-brand-ink',
  '--kn-focus-ring',
  '--kn-brand-accent-border',
  '--kn-brand-accent-rim',
  '--kn-brand-depth-accent',
  '--kn-brand-depth-accent-pressed',
  '--kn-brand-aura',
  '--kn-grad-accent',
  '--kn-grad-accent-hover',
  '--kn-grad-accent-tonal',
  '--kn-on-brand-tonal',
  '--kn-grad-backdrop',
  '--kn-grad-surface-hover',
  '--kn-map-contour',
  '--kn-map-contour-far',
  '--kn-map-aura',
  '--kn-map-glow',
  '--kn-sys-accent',
  '--kn-sys-accent-ink',
  '--kn-sys-accent-tonal',
  '--kn-sys-on-accent-tonal',
] as const;

/** Applies a full accent override on the document root; the default restores the token sheet. */
export function applyAccent(hex: string, dark: boolean) {
  const root = document.documentElement;
  if (hex.toLowerCase() === DEFAULT_ACCENT.toLowerCase()) {
    OVERRIDDEN_PROPERTIES.forEach((property) => root.style.removeProperty(property));
    return;
  }

  const ink = dark ? hex : `color-mix(in oklch, ${hex} 72%, #071A21)`;
  const tint = (percent: number, base: string) => `color-mix(in oklch, ${hex} ${percent}%, ${base})`;
  const alpha = (percent: number) => `color-mix(in oklch, ${hex} ${percent}%, transparent)`;
  const tonal = tint(14, dark ? '#132C35' : '#FFFFFF');
  const values: Record<string, string> = {
    '--kn-accent-fill': hex,
    '--kn-loading': hex,
    '--kn-loading-hover': tint(88, '#FFFFFF'),
    '--kn-loading-ink': ink,
    '--kn-loading-tonal': tonal,
    '--kn-on-loading-tonal': ink,
    '--kn-brand-ink': ink,
    '--kn-focus-ring': ink,
    '--kn-brand-accent-border': alpha(dark ? 30 : 55),
    '--kn-brand-accent-rim':
      `inset 0 1px 1px ${tint(50, '#FFFFFF')}, ` +
      `inset 0 0 0 1px ${dark ? alpha(34) : `color-mix(in oklch, ${ink} 28%, transparent)`}`,
    '--kn-brand-depth-accent':
      `0 ${dark ? 8 : 6}px ${dark ? 24 : 18}px -${dark ? 7 : 5}px ${alpha(54)}, ` +
      '0 1px 2px rgba(5, 31, 39, 0.10)',
    '--kn-brand-depth-accent-pressed': `0 2px 6px -2px ${alpha(35)}`,
    '--kn-brand-aura': `radial-gradient(circle, ${alpha(24)} 0%, ${alpha(7)} 46%, transparent 72%)`,
    '--kn-grad-accent':
      `radial-gradient(in oklch 95% 110% at 18% -10%, ${tint(42, '#FFFFFF')} 0%, transparent 56%), ` +
      `radial-gradient(in oklch 85% 110% at 88% 115%, ${tint(36, '#0080E8')} 0%, transparent 60%)`,
    '--kn-grad-accent-hover':
      `radial-gradient(in oklch 100% 115% at 20% -10%, ${tint(54, '#FFFFFF')} 0%, transparent 58%), ` +
      `radial-gradient(in oklch 85% 110% at 88% 115%, ${tint(40, '#00A0F0')} 0%, transparent 62%)`,
    '--kn-grad-accent-tonal': `radial-gradient(in oklch 110% 90% at 15% 0%, ${tint(dark ? 14 : 65, dark ? hex : '#FFFFFF')} 0%, transparent 68%)`,
    '--kn-on-brand-tonal': ink,
    '--kn-grad-backdrop':
      `radial-gradient(90% 60% at 15% 0%, ${alpha(dark ? 10 : 14)} 0%, transparent 65%), ` +
      `radial-gradient(70% 55% at 88% 100%, ${alpha(dark ? 8 : 12)} 0%, transparent 62%)`,
    '--kn-grad-surface-hover': dark
      ? `radial-gradient(110% 75% at 50% 0%, ${alpha(11)} 0%, transparent 72%)`
      : `radial-gradient(120% 80% at 50% 0%, rgba(255, 255, 255, 0.96) 0%, ${alpha(7)} 100%)`,
    '--kn-map-contour': alpha(22),
    '--kn-map-contour-far': alpha(16),
    '--kn-map-aura': `radial-gradient(circle, ${alpha(dark ? 20 : 18)} 0%, ${alpha(5)} 45%, transparent 72%)`,
    '--kn-map-glow': dark
      ? `radial-gradient(circle at 50% 54%, ${alpha(8)} 0%, rgba(8, 12, 18, 0) 68%)`
      : 'radial-gradient(circle at 50% 54%, rgba(255, 255, 254, 0.65) 0%, rgba(227, 238, 241, 0) 68%)',
    '--kn-sys-accent': hex,
    '--kn-sys-accent-ink': ink,
    '--kn-sys-accent-tonal': tonal,
    '--kn-sys-on-accent-tonal': ink,
  };
  Object.entries(values).forEach(([property, value]) => root.style.setProperty(property, value));
}
