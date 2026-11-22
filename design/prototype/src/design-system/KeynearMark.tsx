import { useId } from 'react';

type StopSpec = readonly [string, string, number?];

type MarkPalette = {
  bg: readonly StopSpec[];
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
  bg: [['0%', '#1E7CFA'], ['52%', '#0656E8'], ['100%', '#0038B8']],
  rim: [['0%', '#80BAFF', 0.45], ['100%', '#002B8F', 0.35]],
  botPillar: [['0%', '#3E82F7'], ['100%', '#7EB2FF']],
  topWing: [['0%', '#6CA6FF'], ['100%', '#C8E2FF']],
  frontRibbon: [['0%', '#FFFFFF'], ['55%', '#EEF5FF'], ['100%', '#C8E0FF']],
  w1: [['0%', '#F2F7FF'], ['100%', '#B8D8FF']],
  w2: [['0%', '#B4D4FF', 0.72], ['100%', '#7EB0FF', 0.45]],
  ambientMatrix: '0 0 0 0 0.00  0 0 0 0 0.08  0 0 0 0 0.32  0 0 0 0.24 0',
  foldMatrix: '0 0 0 0 0.00  0 0 0 0 0.08  0 0 0 0 0.32  0 0 0 0.38 0',
};

const LIGHT_PALETTE: MarkPalette = {
  bg: [['0%', '#F7FAFF'], ['100%', '#E1EBF8']],
  rim: [['0%', '#FFFFFF', 0.9], ['100%', '#C5D6EE', 0.7]],
  botPillar: [['0%', '#002C8F'], ['100%', '#0044CC']],
  topWing: [['0%', '#1458DC'], ['100%', '#5E9EFF']],
  frontRibbon: [['0%', '#68ADFF'], ['48%', '#1E74F6'], ['100%', '#004CED']],
  w1: [['0%', '#5296F7'], ['100%', '#1A66E8']],
  w2: [['0%', '#8AB6F9', 0.85], ['100%', '#5A92EE', 0.55]],
  ambientMatrix: '0 0 0 0 0.02  0 0 0 0 0.08  0 0 0 0 0.24  0 0 0 0.16 0',
  foldMatrix: '0 0 0 0 0.02  0 0 0 0 0.08  0 0 0 0 0.24  0 0 0 0.28 0',
};

const DARK_PALETTE: MarkPalette = {
  bg: [['0%', '#13223D'], ['100%', '#0A1324']],
  rim: [['0%', '#4A74B8', 0.3], ['100%', '#1A3260', 0.2]],
  botPillar: [['0%', '#102C6E'], ['100%', '#1E4DB8']],
  topWing: [['0%', '#1948AD'], ['100%', '#4C8DFF']],
  frontRibbon: [['0%', '#8EC0FF'], ['50%', '#458DFF'], ['100%', '#1B64E8']],
  w1: [['0%', '#78B0FF'], ['100%', '#3578F0']],
  w2: [['0%', '#4C84E0', 0.65], ['100%', '#2452B0', 0.4]],
  ambientMatrix: '0 0 0 0 0.01  0 0 0 0 0.02  0 0 0 0 0.07  0 0 0 0.45 0',
  foldMatrix: '0 0 0 0 0.01  0 0 0 0 0.02  0 0 0 0 0.07  0 0 0 0.58 0',
};

function renderStops(stops: readonly StopSpec[]) {
  return stops.map(([offset, color, opacity]) => (
    <stop key={offset} offset={offset} stopColor={color} stopOpacity={opacity} />
  ));
}

function PaletteDefs({ prefix, palette }: { prefix: string; palette: MarkPalette }) {
  return (
    <>
      <linearGradient id={`${prefix}-bg`} x1="0.15" y1="0" x2="0.85" y2="1">
        {renderStops(palette.bg)}
      </linearGradient>
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

function MarkGeometry({ prefix, className }: { prefix: string; className?: string }) {
  return (
    <g className={className}>
      <rect x="24" y="24" width="464" height="464" rx="112" fill={`url(#${prefix}-bg)`} />
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
  variant?: 'brand' | 'adaptive' | 'symbolic';
  className?: string;
}) {
  const uid = useId().replace(/:/g, '');
  const brandPrefix = `${uid}-b`;
  const lightPrefix = `${uid}-l`;
  const darkPrefix = `${uid}-d`;

  if (variant === 'symbolic') {
    return (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width={size}
        height={size}
        viewBox="116 96 300 316"
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
        {variant === 'brand' ? (
          <PaletteDefs prefix={brandPrefix} palette={BRAND_PALETTE} />
        ) : (
          <>
            <PaletteDefs prefix={lightPrefix} palette={LIGHT_PALETTE} />
            <PaletteDefs prefix={darkPrefix} palette={DARK_PALETTE} />
          </>
        )}
      </defs>
      {variant === 'brand' ? (
        <MarkGeometry prefix={brandPrefix} />
      ) : (
        <>
          <MarkGeometry prefix={lightPrefix} className="kn-when-light" />
          <MarkGeometry prefix={darkPrefix} className="kn-when-dark" />
        </>
      )}
    </svg>
  );
}
