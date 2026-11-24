export function ProximityHalo() {
  return (
    <svg
      viewBox="0 0 420 420"
      aria-hidden="true"
      className="kn-halo pointer-events-none absolute left-1/2 top-1/2 h-[260px] w-[260px] -translate-x-1/2 -translate-y-1/2 sm:h-[360px] sm:w-[360px] lg:h-[420px] lg:w-[420px]"
    >
      <circle cx="210" cy="210" r="124" fill="none" stroke="var(--kn-accent)" strokeOpacity="0.24" strokeWidth="1.5" />
      <circle cx="210" cy="210" r="168" fill="none" stroke="var(--kn-accent)" strokeOpacity="0.18" strokeWidth="1.5" strokeDasharray="6 10" />
      <circle cx="210" cy="210" r="206" fill="none" stroke="var(--kn-accent)" strokeOpacity="0.12" strokeWidth="1.5" />
      <g className="kn-halo-pulse">
        <circle cx="356" cy="126" r="11" fill="var(--kn-accent)" fillOpacity="0.24" />
        <circle cx="356" cy="126" r="5" fill="var(--kn-accent)" />
      </g>
      <g className="kn-halo-pulse-delayed">
        <circle cx="64" cy="294" r="9" fill="var(--kn-accent)" fillOpacity="0.2" />
        <circle cx="64" cy="294" r="4" fill="var(--kn-accent)" />
      </g>
    </svg>
  );
}
