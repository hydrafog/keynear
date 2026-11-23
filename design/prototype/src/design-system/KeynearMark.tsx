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
    <g fill="currentColor">
      <path opacity="0.55" d="m 160,276 c 17.728,0 32,14.272 32,32 v 60 c 0,17.728 -14.272,32 -32,32 -17.728,0 -32,-14.272 -32,-32 v -60 c 0,-17.728 14.272,-32 32,-32 z" />
      <path opacity="0.75" d="m 324.30273,104.08203 a 32,32 0 0 0 -23.24218,7.7207 l -104,90 a 32,32 0 0 0 -3.25782,45.13672 32,32 0 0 0 45.13672,3.25782 l 104,-90 a 32,32 0 0 0 3.25782,-45.13672 32,32 0 0 0 -21.89454,-10.97852 z" />
      <path d="m 160,112 a 32,32 0 0 0 -32,32 v 104 c 0,23.39394 11.94155,46.93935 30.35352,61.25977 a 32.0032,32.0032 0 0 0 2.68554,1.87695 l 144,90 a 32,32 0 0 0 44.09766,-10.17578 32,32 0 0 0 -10.17578,-44.09766 L 196.6582,257.92383 C 192.24605,254.4069 192,253.48645 192,248 V 144 a 32,32 0 0 0 -32,-32 z" />
      <path d="m 335.31055,191.02148 a 11,11 0 0 0 -7.56055,3.70313 11,11 0 0 0 0.97461,15.52539 c 27.67868,24.4103 27.67868,67.0897 0,91.5 A 11,11 0 0 0 327.75,317.27539 11,11 0 0 0 343.27539,318.25 c 37.32525,-32.91778 37.32525,-91.58222 0,-124.5 a 11,11 0 0 0 -7.96484,-2.72852 z" />
      <path opacity="0.58" d="M 375.49805,164.01562 A 8,8 0 0 0 370,166.70898 8,8 0 0 0 370.70898,178 c 47.05078,41.49488 47.05078,114.50512 0,156 A 8,8 0 0 0 370,345.29102 8,8 0 0 0 381.29102,346 c 54.06646,-47.68214 54.06646,-132.31786 0,-180 a 8,8 0 0 0 -5.79297,-1.98438 z" />
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
        viewBox="108 96 316 316"
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
