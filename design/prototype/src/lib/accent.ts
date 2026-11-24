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

/** Applies the chosen accent color uniformly across all presets and custom picks. */
export function applyAccent(hex: string, _dark: boolean) {
  const root = document.documentElement;
  root.style.setProperty('--kn-accent-fill', hex);

  const metaTheme = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');
  if (metaTheme) {
    metaTheme.content = hex;
  }
}
