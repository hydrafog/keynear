import { useEffect, useRef } from 'react';

const WAVE = {
  spokes: 12,
  ringPoints: 96,
  baseLobes: 6,
  lobeStep: 2,
  hiddenAmp: 0.5,
  baseAmp: 2.2,
  ampStep: 0.9,
  spokeRate: 0.00015,
  waveRate: 0.0028,
  secHarmonic: 3,
  secWaveFactor: 0.7,
  secAmpFactor: 0.5,
  echoAmpFactor: 1.15,
  echoPhaseFactor: 1.1,
  moteRate: 0.0004,
  moteWaveRate: 0.002,
  moteCount: 8,
  moteDrift: 6,
  moteRadius: 1.5,
  cycleMs: 120_000,
  /** Time for the radar to ramp between hidden and active. */
  fadeMs: 800,
  /** Spokes and motes dim to this share of their alpha while hidden. */
  idleAlpha: 0.4,
  ratios: [78 / 196, 136 / 196, 1] as const,
} as const;

interface WavePalette {
  spoke: string;
  ring: string;
  echo: string;
  mote: string;
}

type Rgb = readonly [number, number, number];

function parseHex(hex: string): Rgb {
  const value = hex.replace('#', '');
  return [
    parseInt(value.slice(0, 2), 16),
    parseInt(value.slice(2, 4), 16),
    parseInt(value.slice(4, 6), 16),
  ];
}

function mixRgb(from: Rgb, to: Rgb, amount: number): Rgb {
  return [
    Math.round(from[0] + (to[0] - from[0]) * amount),
    Math.round(from[1] + (to[1] - from[1]) * amount),
    Math.round(from[2] + (to[2] - from[2]) * amount),
  ];
}

const rgba = (rgb: Rgb, alpha: number) => `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${alpha})`;

function paletteFor(accent: string, dark: boolean): WavePalette {
  const base = parseHex(accent);
  const light = mixRgb(base, [255, 255, 255], 0.5);
  const ink = mixRgb(base, [7, 26, 33], 0.45);
  return dark
    ? { spoke: rgba(light, 0.12), ring: rgba(base, 0.38), echo: rgba(light, 0.22), mote: rgba(light, 0.44) }
    : { spoke: rgba(ink, 0.12), ring: rgba(ink, 0.22), echo: rgba(base, 0.28), mote: rgba(ink, 0.36) };
}

function drawRingWave(
  ctx: CanvasRenderingContext2D, cx: number, cy: number, baseR: number,
  lobes: number, amp: number, phase: number, isEcho: boolean
) {
  const twoPi = Math.PI * 2;
  ctx.beginPath();
  for (let p = 0; p <= WAVE.ringPoints; p += 1) {
    const theta = (p / WAVE.ringPoints) * twoPi;
    const wave = isEcho
      ? Math.cos(theta * lobes - phase * WAVE.echoPhaseFactor) * (amp * WAVE.echoAmpFactor)
      : Math.sin(theta * lobes + phase) * amp +
        Math.cos(theta * WAVE.secHarmonic - phase * WAVE.secWaveFactor) * (amp * WAVE.secAmpFactor);
    const r = baseR + wave;
    const x = cx + Math.cos(theta) * r;
    const y = cy + Math.sin(theta) * r;
    if (p === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.closePath();
  ctx.stroke();
}

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const smoothstep = (t: number) => t * t * (3 - 2 * t);

/** `intensity` runs from 0 (hidden, calm) to 1 (listening, lively) and is eased before use. */
function drawWaveFrame(
  ctx: CanvasRenderingContext2D, w: number, h: number, t: number,
  intensity: number, palette: WavePalette
) {
  const e = smoothstep(intensity);
  ctx.clearRect(0, 0, w, h);
  const cx = w / 2;
  const cy = h / 2;
  const maxR = Math.max(110, Math.min(w, h) * 0.46);
  const guideRadii = WAVE.ratios.map((ratio) => maxR * ratio);
  const twoPi = Math.PI * 2;
  const ambientAlpha = lerp(WAVE.idleAlpha, 1, e);

  ctx.globalAlpha = ambientAlpha;
  ctx.strokeStyle = palette.spoke;
  ctx.lineWidth = 1;
  for (let s = 0; s < WAVE.spokes; s += 1) {
    const a = (s / WAVE.spokes) * twoPi + t * WAVE.spokeRate;
    ctx.beginPath();
    ctx.moveTo(cx + Math.cos(a) * guideRadii[0], cy + Math.sin(a) * guideRadii[0]);
    ctx.lineTo(cx + Math.cos(a) * maxR, cy + Math.sin(a) * maxR);
    ctx.stroke();
  }

  guideRadii.forEach((baseR, idx) => {
    const lobes = WAVE.baseLobes + idx * WAVE.lobeStep;
    const amp = lerp(WAVE.hiddenAmp, WAVE.baseAmp + idx * WAVE.ampStep, e);
    const phase = t * WAVE.waveRate * (idx % 2 === 0 ? 1 : -1) + idx;
    ctx.globalAlpha = 1;
    ctx.strokeStyle = palette.ring;
    ctx.lineWidth = 2;
    drawRingWave(ctx, cx, cy, baseR, lobes, amp, phase, false);
    if (e > 0.01) {
      ctx.globalAlpha = e;
      ctx.strokeStyle = palette.echo;
      ctx.lineWidth = 4.5;
      drawRingWave(ctx, cx, cy, baseR, lobes, amp, phase, true);
    }
  });

  ctx.globalAlpha = ambientAlpha;
  ctx.fillStyle = palette.mote;
  guideRadii.forEach((baseR, idx) => {
    for (let s = 0; s < WAVE.moteCount; s += 1) {
      const theta = (s / WAVE.moteCount) * twoPi + t * WAVE.moteRate + idx;
      const r = baseR + Math.sin(t * WAVE.moteWaveRate + idx + s) * WAVE.moteDrift;
      ctx.beginPath();
      ctx.arc(cx + Math.cos(theta) * r, cy + Math.sin(theta) * r, WAVE.moteRadius, 0, twoPi);
      ctx.fill();
    }
  });
  ctx.globalAlpha = 1;
}

export function WaveMapCanvas({ active, darkMode, accent }: { active: boolean; darkMode: boolean; accent: string }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const activeRef = useRef(active);
  const wakeRef = useRef<(() => void) | null>(null);
  const startRef = useRef(0);

  // Visibility changes only retarget the intensity; the loop below eases toward it.
  useEffect(() => {
    activeRef.current = active;
    wakeRef.current?.();
  }, [active]);

  useEffect(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext('2d');
    if (!canvas || !ctx) return;

    const palette = paletteFor(accent, darkMode);
    const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
    if (startRef.current === 0) startRef.current = performance.now();
    const startTime = startRef.current;
    const target = () => (activeRef.current ? 1 : 0);

    let rafId: number | null = null;
    let width = 1;
    let height = 1;
    let intensity = target();
    let lastNow = performance.now();

    const syncSize = () => {
      const rect = canvas.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      width = Math.max(1, Math.round(rect.width));
      height = Math.max(1, Math.round(rect.height));
      canvas.width = Math.round(width * dpr);
      canvas.height = Math.round(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const draw = (now: number) =>
      drawWaveFrame(ctx, width, height, (now - startTime) % WAVE.cycleMs, intensity, palette);

    const frame = (now: number) => {
      const goal = target();
      const step = Math.min(64, now - lastNow) / WAVE.fadeMs;
      lastNow = now;
      intensity = goal > intensity ? Math.min(goal, intensity + step) : Math.max(goal, intensity - step);
      draw(now);
      // Keep animating while listening or while easing down; settle on a still frame when hidden.
      rafId = goal === 1 || intensity > 0 ? requestAnimationFrame(frame) : null;
    };

    const wake = () => {
      if (reducedMotion.matches) {
        intensity = target();
        draw(performance.now());
        return;
      }
      if (rafId !== null) return;
      lastNow = performance.now();
      rafId = requestAnimationFrame(frame);
    };
    wakeRef.current = wake;

    syncSize();
    if (reducedMotion.matches || target() === 0) draw(performance.now());
    else wake();

    const observer = new ResizeObserver(() => {
      syncSize();
      if (rafId === null) draw(performance.now());
    });
    observer.observe(canvas);
    reducedMotion.addEventListener('change', wake);

    return () => {
      if (rafId !== null) cancelAnimationFrame(rafId);
      wakeRef.current = null;
      observer.disconnect();
      reducedMotion.removeEventListener('change', wake);
    };
  }, [darkMode, accent]);

  return <canvas ref={canvasRef} aria-hidden="true" className="pointer-events-none absolute inset-0 h-full w-full" />;
}
