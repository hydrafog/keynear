export interface Orbit {
  radius: number;
  duration: number;
  phase: number;
}

const DURATION = 40;

export function orbitFor(rssi: number, idx: number, count: number): Orbit {
  const strength = Math.min(1, Math.max(0, (-rssi - 40) / 50));
  return {
    radius: 128 + strength * 48,
    duration: DURATION,
    phase: (idx / Math.max(1, count)) * DURATION,
  };
}
