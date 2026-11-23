import type { ReactNode } from 'react';

const CARD_TONE = {
  surface: 'kn-surface',
  tonal: 'kn-surface-tonal',
} as const;

/** Standalone content card: 16px padding, kn-2xl radius. Pass `label` to render a labelled region. */
export function Card({
  tone = 'surface',
  label,
  className = '',
  children,
}: {
  tone?: keyof typeof CARD_TONE;
  label?: string;
  className?: string;
  children: ReactNode;
}) {
  const classes = `rounded-kn-2xl p-4 ${CARD_TONE[tone]} ${className}`;
  return label ? (
    <section aria-label={label} className={classes}>{children}</section>
  ) : (
    <div className={classes}>{children}</div>
  );
}

/** Grouped list container: 8px padding so rows (px-4 py-3) align with Card content. */
export function ListGroup({ className = '', children }: { className?: string; children: ReactNode }) {
  return <div className={`space-y-1.5 rounded-kn-2xl p-2 kn-surface ${className}`}>{children}</div>;
}
