export interface Orbit {
  radius: number;
  duration: number;
  phase: number;
}

export interface RadarLayout {
  /** Uniform scale applied to the beacon and pins so the scene fits the radar viewport. */
  scale: number;
  /** Largest orbit radius, in unscaled pixels. */
  maxRadius: number;
}

const DURATION = 40;

/** Half of a pin label's widest case (136px text + 28px padding). Outer pins stay inset by this much. */
const PIN_HALF_WIDTH = 82;
/** Half of a pin height (44px avatar + two-line label card). */
const PIN_HALF_HEIGHT = 50;
/** Smallest orbit that still clears the centre beacon. */
const MIN_RADIUS = 116;
const MAX_RADIUS = 172;
const MIN_SCALE = 0.62;
const INNER_RATIO = 0.8;

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

export function radarLayoutFor(width: number, height: number): RadarLayout {
  if (width <= 0 || height <= 0) return { scale: 1, maxRadius: MIN_RADIUS };
  const fitH = height / (2 * (MIN_RADIUS + PIN_HALF_HEIGHT));
  const fitW = width / (2 * (MIN_RADIUS + PIN_HALF_WIDTH));
  const scale = clamp(Math.min(fitH, fitW, 1), MIN_SCALE, 1);
  const maxRadius = clamp(
    Math.min(width / (2 * scale) - PIN_HALF_WIDTH, height / (2 * scale) - PIN_HALF_HEIGHT),
    MIN_RADIUS,
    MAX_RADIUS
  );
  return { scale, maxRadius };
}

export function orbitFor(rssi: number, idx: number, count: number, layout: RadarLayout): Orbit {
  // 0 for a close, strong signal; 1 for a weak, distant one. Weaker signals sit on the outer orbit.
  const distance = Math.min(1, Math.max(0, (-rssi - 40) / 50));
  const inner = Math.max(MIN_RADIUS, layout.maxRadius * INNER_RATIO);
  return {
    radius: inner + distance * (layout.maxRadius - inner),
    duration: DURATION,
    phase: (idx / Math.max(1, count)) * DURATION,
  };
}
