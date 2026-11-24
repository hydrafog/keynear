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
/** Half of a pin height (58px avatar + two-line label card). */
const PIN_HALF_HEIGHT = 57;
/** Smallest orbit that still clears the centre beacon. */
const MIN_RADIUS = 116;
const MAX_RADIUS = 172;
const MIN_SCALE = 0.62;
const INNER_RATIO = 0.8;
const RING_COUNT = 3;

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

export function orbitPlacementsFor(deviceIds: string[], layout: RadarLayout): Orbit[] {
  const count = deviceIds.length;
  if (count === 0) return [];
  const rankById = deviceIds.map((_, index) => index).sort((a, b) =>
    deviceIds[a] < deviceIds[b] ? -1 : deviceIds[a] > deviceIds[b] ? 1 : 0
  );
  const ringOf = new Array<number>(count).fill(0);
  rankById.forEach((index, rank) => {
    ringOf[index] = count === 1 ? 1 : count === 2 ? (rank === 0 ? 0 : 2) : rank % RING_COUNT;
  });
  const ringCounts = new Array<number>(RING_COUNT).fill(0);
  ringOf.forEach((ring) => {
    ringCounts[ring] += 1;
  });
  const slotOf = new Array<number>(count).fill(0);
  const nextSlot = new Array<number>(RING_COUNT).fill(0);
  rankById.forEach((index) => {
    const ring = ringOf[index];
    slotOf[index] = nextSlot[ring];
    nextSlot[ring] += 1;
  });
  const inner = Math.max(MIN_RADIUS, layout.maxRadius * INNER_RATIO);
  return deviceIds.map((_, index) => {
    const ring = ringOf[index];
    const radius = inner + ((layout.maxRadius - inner) * ring) / (RING_COUNT - 1);
    const phase = ((slotOf[index] / ringCounts[ring] + ring / RING_COUNT) % 1) * DURATION;
    return { radius: Math.round(radius), duration: DURATION, phase };
  });
}
