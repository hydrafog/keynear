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
  ratios: [78 / 196, 136 / 196, 1] as const,
} as const;

interface WavePalette {
  spoke: string;
  ring: string;
  echo: string;
  mote: string;
}

const LIGHT_PALETTE: WavePalette = {
  spoke: 'rgba(30, 116, 246, 0.13)',
  ring: 'rgba(6, 86, 232, 0.24)',
  echo: 'rgba(104, 173, 255, 0.22)',
  mote: 'rgba(30, 124, 250, 0.36)',
};

const DARK_PALETTE: WavePalette = {
  spoke: 'rgba(120, 176, 255, 0.14)',
  ring: 'rgba(69, 141, 255, 0.34)',
  echo: 'rgba(142, 192, 255, 0.22)',
  mote: 'rgba(142, 192, 255, 0.44)',
};

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

function drawWaveFrame(
  ctx: CanvasRenderingContext2D, w: number, h: number, t: number,
  active: boolean, palette: WavePalette
) {
  ctx.clearRect(0, 0, w, h);
  const cx = w / 2;
  const cy = h / 2;
  const maxR = Math.max(110, Math.min(w, h) * 0.46);
  const guideRadii = WAVE.ratios.map((ratio) => maxR * ratio);
  const twoPi = Math.PI * 2;

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
    const amp = active ? WAVE.baseAmp + idx * WAVE.ampStep : WAVE.hiddenAmp;
    const phase = t * WAVE.waveRate * (idx % 2 === 0 ? 1 : -1) + idx;
    ctx.strokeStyle = palette.ring;
    ctx.lineWidth = 2;
    drawRingWave(ctx, cx, cy, baseR, lobes, amp, phase, false);
    if (active) {
      ctx.strokeStyle = palette.echo;
      ctx.lineWidth = 4.5;
      drawRingWave(ctx, cx, cy, baseR, lobes, amp, phase, true);
    }
  });

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
}

export function WaveMapCanvas({ active, darkMode }: { active: boolean; darkMode: boolean }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext('2d');
    if (!canvas || !ctx) return;

    const palette = darkMode ? DARK_PALETTE : LIGHT_PALETTE;
    const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
    let rafId: number | null = null;
    let width = 1;
    let height = 1;
    let startTime = performance.now();

    const syncSize = () => {
      const rect = canvas.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      width = Math.max(1, Math.round(rect.width));
      height = Math.max(1, Math.round(rect.height));
      canvas.width = Math.round(width * dpr);
      canvas.height = Math.round(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const renderLoop = (now: number) => {
      drawWaveFrame(ctx, width, height, (now - startTime) % WAVE.cycleMs, active, palette);
      rafId = requestAnimationFrame(renderLoop);
    };

    const startOrDrawStatic = () => {
      if (rafId !== null) cancelAnimationFrame(rafId);
      rafId = null;
      if (reducedMotion.matches) {
        drawWaveFrame(ctx, width, height, 0, active, palette);
        return;
      }
      startTime = performance.now();
      rafId = requestAnimationFrame(renderLoop);
    };

    syncSize();
    startOrDrawStatic();

    const observer = new ResizeObserver(() => {
      syncSize();
      if (reducedMotion.matches) drawWaveFrame(ctx, width, height, 0, active, palette);
    });
    observer.observe(canvas);
    reducedMotion.addEventListener('change', startOrDrawStatic);

    return () => {
      if (rafId !== null) cancelAnimationFrame(rafId);
      observer.disconnect();
      reducedMotion.removeEventListener('change', startOrDrawStatic);
    };
  }, [active, darkMode]);

  return <canvas ref={canvasRef} aria-hidden="true" className="pointer-events-none absolute inset-0 h-full w-full" />;
}
