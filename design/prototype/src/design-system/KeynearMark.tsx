import { useId } from 'react';

type StopSpec = readonly [string, string, number?];

type MarkPalette = {
  base: string;
  sheen: readonly StopSpec[];
  shift: readonly StopSpec[];
  rim: readonly StopSpec[];
  botPillar: readonly StopSpec[];
  topWing: readonly StopSpec[];
  frontRibbon: readonly StopSpec[];
  w1: readonly StopSpec[];
  w2: readonly StopSpec[];
  ambientMatrix: string;
  foldMatrix: string;
};

const BRAND_PALETTE: MarkPalette = {
  base: '#4ABBD5',
  sheen: [['0%', '#8AE8F5', 0.85], ['100%', '#8AE8F5', 0]],
  shift: [['0%', '#4BA6DF', 0.75], ['100%', '#4BA6DF', 0]],
  rim: [['0%', '#C3F4FB', 0.55], ['100%', '#0E748A', 0.30]],
  botPillar: [['0%', '#2F9FBC'], ['100%', '#8ADCEC']],
  topWing: [['0%', '#7FD4E8'], ['100%', '#D6F3FA']],
  frontRibbon: [['0%', '#FFFFFF'], ['100%', '#FFFFFF']],
  w1: [['0%', '#F4FCFE'], ['100%', '#C4EBF4']],
  w2: [['0%', '#D9F2F8', 0.78], ['100%', '#A8DFEC', 0.5]],
  ambientMatrix: '0 0 0 0 0.00  0 0 0 0 0.17  0 0 0 0 0.24  0 0 0 0.24 0',
  foldMatrix: '0 0 0 0 0.00  0 0 0 0 0.17  0 0 0 0 0.24  0 0 0 0.36 0',
};

function renderStops(stops: readonly StopSpec[]) {
  return stops.map(([offset, color, opacity]) => (
    <stop key={offset} offset={offset} stopColor={color} stopOpacity={opacity} />
  ));
}

function PaletteDefs({ prefix, palette }: { prefix: string; palette: MarkPalette }) {
  return (
    <>
      <radialGradient id={`${prefix}-sheen`} cx="0.2" cy="0.1" r="0.75">
        {renderStops(palette.sheen)}
      </radialGradient>
      <radialGradient id={`${prefix}-shift`} cx="0.9" cy="0.95" r="0.7">
        {renderStops(palette.shift)}
      </radialGradient>
      <linearGradient id={`${prefix}-rim`} x1="0" y1="0" x2="0" y2="1">
        {renderStops(palette.rim)}
      </linearGradient>
      <linearGradient id={`${prefix}-bot`} x1="0.5" y1="0" x2="0.5" y2="1">
        {renderStops(palette.botPillar)}
      </linearGradient>
      <linearGradient id={`${prefix}-wing`} x1="0.15" y1="0.85" x2="0.9" y2="0.1">
        {renderStops(palette.topWing)}
      </linearGradient>
      <linearGradient id={`${prefix}-ribbon`} x1="0.15" y1="0.08" x2="0.88" y2="0.92">
        {renderStops(palette.frontRibbon)}
      </linearGradient>
      <linearGradient id={`${prefix}-w1`} x1="0" y1="0" x2="0" y2="1">
        {renderStops(palette.w1)}
      </linearGradient>
      <linearGradient id={`${prefix}-w2`} x1="0" y1="0" x2="0" y2="1">
        {renderStops(palette.w2)}
      </linearGradient>
      <filter id={`${prefix}-amb`} x="-30%" y="-30%" width="160%" height="160%">
        <feGaussianBlur in="SourceAlpha" stdDeviation="7" result="blur" />
        <feOffset in="blur" dx="0" dy="5" result="offsetBlur" />
        <feColorMatrix in="offsetBlur" type="matrix" values={palette.ambientMatrix} result="shadow" />
        <feMerge>
          <feMergeNode in="shadow" />
          <feMergeNode in="SourceGraphic" />
        </feMerge>
      </filter>
      <filter id={`${prefix}-fold`} x="-35%" y="-35%" width="170%" height="170%">
        <feGaussianBlur in="SourceAlpha" stdDeviation="9" result="blur" />
        <feOffset in="blur" dx="-2" dy="6" result="offsetBlur" />
        <feColorMatrix in="offsetBlur" type="matrix" values={palette.foldMatrix} result="shadow" />
        <feMerge>
          <feMergeNode in="shadow" />
          <feMergeNode in="SourceGraphic" />
        </feMerge>
      </filter>
    </>
  );
}

function MarkGeometry({ prefix, palette }: { prefix: string; palette: MarkPalette }) {
  return (
    <g>
      <rect x="24" y="24" width="464" height="464" rx="112" fill={palette.base} />
      <rect x="24" y="24" width="464" height="464" rx="112" fill={`url(#${prefix}-sheen)`} />
      <rect x="24" y="24" width="464" height="464" rx="112" fill={`url(#${prefix}-shift)`} />
      <rect x="25.5" y="25.5" width="461" height="461" rx="110.5" fill="none" stroke={`url(#${prefix}-rim)`} strokeWidth="3" />
      <g filter={`url(#${prefix}-amb)`}>
        <path d="M 336 202 A 72 72 0 0 1 336 310" fill="none" stroke={`url(#${prefix}-w1)`} strokeWidth="22" strokeLinecap="round" />
        <path d="M 376 172 A 112 112 0 0 1 376 340" fill="none" stroke={`url(#${prefix}-w2)`} strokeWidth="16" strokeLinecap="round" />
      </g>
      <g filter={`url(#${prefix}-amb)`}>
        <rect x="128" y="232" width="64" height="168" rx="32" fill={`url(#${prefix}-bot)`} />
        <path d="M 174 264 L 322 136" fill="none" stroke={`url(#${prefix}-wing)`} strokeWidth="64" strokeLinecap="round" />
        <g filter={`url(#${prefix}-fold)`}>
          <path
            d="M 160 144 L 160 248 Q 160 270 178 284 L 322 374"
            fill="none"
            stroke={`url(#${prefix}-ribbon)`}
            strokeWidth="64"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </g>
      </g>
    </g>
  );
}

function SymbolicGeometry() {
  return (
    <g>
      <circle cx="232" cy="290" r="88" fill="currentColor" fillOpacity="0.08" />
      <g fill="none" stroke="currentColor" strokeLinecap="round">
        <path d="M 60.4 146.0 A 224 224 0 0 1 330.2 88.7" strokeWidth="5" strokeOpacity="0.18" />
        <path d="M 452.6 251.1 A 224 224 0 0 1 393.1 445.6" strokeWidth="5" strokeOpacity="0.18" />
        <path d="M 389.9 238.7 A 166 166 0 0 1 398.0 287.1" strokeWidth="6" strokeOpacity="0.24" />
        <path d="M 371.2 380.4 A 166 166 0 0 1 86.8 209.5" strokeWidth="6" strokeOpacity="0.24" />
        <path d="M 339.6 246.5 A 116 116 0 1 1 118.5 265.9" strokeWidth="6" strokeOpacity="0.30" />
      </g>
      <g fill="none" stroke="currentColor" strokeLinecap="round">
        <path d="M 101.2 187.8 A 166 166 0 0 1 272.2 128.9" strokeWidth="18" strokeOpacity="0.68" />
        <path d="M 127.7 239.1 A 116 116 0 0 1 267.8 179.7" strokeWidth="22" strokeOpacity="0.88" />
      </g>
      <circle cx="391.6" cy="335.8" r="34" fill="currentColor" fillOpacity="0.22" />
      <circle cx="391.6" cy="335.8" r="18" fill="currentColor" fillOpacity="0.96" />
      <g fill="none" stroke="currentColor" strokeWidth="42" strokeLinecap="round" strokeLinejoin="round">
        <circle cx="232" cy="290" r="54" />
        <path d="M 270.2 251.8 L 387.6 134.4 L 424.3 171.2" />
        <path d="M 339.5 182.5 L 364.9 208.0" />
      </g>
    </g>
  );
}

export function KeynearMark({
  size = 28,
  variant = 'brand',
  className = '',
}: {
  size?: number;
  variant?: 'brand' | 'symbolic';
  className?: string;
}) {
  const prefix = `${useId().replace(/:/g, '')}-b`;

  if (variant === 'symbolic') {
    return (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width={size}
        height={size}
        viewBox="0 0 512 512"
        aria-hidden="true"
        className={className}
      >
        <SymbolicGeometry />
      </svg>
    );
  }

  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 512 512"
      aria-hidden="true"
      className={className}
    >
      <defs>
        <PaletteDefs prefix={prefix} palette={BRAND_PALETTE} />
      </defs>
      <MarkGeometry prefix={prefix} palette={BRAND_PALETTE} />
    </svg>
  );
}
