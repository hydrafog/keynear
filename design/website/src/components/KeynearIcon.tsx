import { useId } from 'react';

export interface KeynearIconProps {
  size?: number;
  className?: string;
}

export function KeynearIcon({ size = 32, className = '' }: KeynearIconProps) {
  const id = useId().replace(/:/g, '');

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
        <clipPath id={`${id}-clip`}>
          <rect x="24" y="24" width="464" height="464" rx="112" />
        </clipPath>
        <radialGradient id={`${id}-sheen`} cx="0.18" cy="0.08" r="0.78">
          <stop offset="0%" stopColor="#96F0FA" stopOpacity="0.88" />
          <stop offset="100%" stopColor="#8AE8F5" stopOpacity="0" />
        </radialGradient>
        <radialGradient id={`${id}-shift`} cx="0.92" cy="0.96" r="0.78">
          <stop offset="0%" stopColor="#2F8ED0" stopOpacity="0.72" />
          <stop offset="100%" stopColor="#4BA6DF" stopOpacity="0" />
        </radialGradient>
        <linearGradient id={`${id}-rim`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#D2F8FD" stopOpacity="0.62" />
          <stop offset="100%" stopColor="#0A6A7E" stopOpacity="0.32" />
        </linearGradient>
        <radialGradient id={`${id}-aura`} cx="198" cy="314" r="156" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="#C8FAFF" stopOpacity="0.28" />
          <stop offset="60%" stopColor="#8AE8F5" stopOpacity="0.08" />
          <stop offset="100%" stopColor="#8AE8F5" stopOpacity="0" />
        </radialGradient>
        <linearGradient id={`${id}-w1`} x1="94" y1="263" x2="234" y2="204" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="#C6EEF8" stopOpacity="0.84" />
          <stop offset="100%" stopColor="#F7FEFF" stopOpacity="0.96" />
        </linearGradient>
        <linearGradient id={`${id}-w2`} x1="67" y1="212" x2="238" y2="153" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="#9DDEF0" stopOpacity="0.52" />
          <stop offset="100%" stopColor="#E2F8FD" stopOpacity="0.80" />
        </linearGradient>
        <linearGradient id={`${id}-peer`} x1="340" y1="342" x2="376" y2="378" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="#FFFFFF" />
          <stop offset="100%" stopColor="#C8EFF8" />
        </linearGradient>
        <linearGradient id={`${id}-key`} x1="144" y1="148" x2="396" y2="372" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="#FFFFFF" />
          <stop offset="100%" stopColor="#E4F9FD" />
        </linearGradient>
      </defs>

      <rect x="24" y="24" width="464" height="464" rx="112" fill="#4ABBD5" />
      <rect x="24" y="24" width="464" height="464" rx="112" fill={`url(#${id}-sheen)`} />
      <rect x="24" y="24" width="464" height="464" rx="112" fill={`url(#${id}-shift)`} />

      <g clipPath={`url(#${id}-clip)`}>
        <circle cx="198" cy="314" r="156" fill={`url(#${id}-aura)`} />
        <circle cx="198" cy="314" r="116" fill="none" stroke="#E8FCFF" strokeOpacity="0.22" strokeWidth="2.5" />
        <circle cx="198" cy="314" r="166" fill="none" stroke="#E8FCFF" strokeOpacity="0.17" strokeWidth="2.5" />
        <circle cx="198" cy="314" r="224" fill="none" stroke="#E8FCFF" strokeOpacity="0.11" strokeWidth="2" />
      </g>

      <rect x="25.5" y="25.5" width="461" height="461" rx="110.5" fill="none" stroke={`url(#${id}-rim)`} strokeWidth="3" />

      <g>
        <path d="M 93.7 263.1 A 116 116 0 0 1 233.8 203.7" fill="none" stroke={`url(#${id}-w1)`} strokeWidth="22" strokeLinecap="round" />
        <path d="M 67.2 211.8 A 166 166 0 0 1 238.2 152.9" fill="none" stroke={`url(#${id}-w2)`} strokeWidth="18" strokeLinecap="round" />
        <circle cx="357.6" cy="359.8" r="34" fill="#DDF8FD" fillOpacity="0.24" />
        <circle cx="357.6" cy="359.8" r="18" fill={`url(#${id}-peer)`} />
      </g>

      <g fill="none" stroke={`url(#${id}-key)`} strokeWidth="42" strokeLinecap="round" strokeLinejoin="round">
        <circle cx="198" cy="314" r="54" />
        <path d="M 236.2 275.8 L 353.6 158.4 L 390.3 195.2" />
        <path d="M 305.5 206.5 L 330.9 232.0" />
      </g>
    </svg>
  );
}
