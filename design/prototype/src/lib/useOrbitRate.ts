import { useEffect, useRef, type RefObject } from 'react';

const ORBIT_ANIMATIONS = new Set(['kn-orbit', 'kn-orbit-counter']);
const RAMP_MS = 800;

/**
 * Eases the orbit animations under `ref` between full speed and a stop, instead of
 * freezing them mid-step. Orbit and counter-orbit share one rate so pins stay upright.
 * `pinCount` re-applies the current rate when pins mount or unmount.
 */
export function useOrbitRate(ref: RefObject<HTMLElement | null>, moving: boolean, pinCount: number) {
  const rateRef = useRef(moving ? 1 : 0);

  useEffect(() => {
    const target = moving ? 1 : 0;
    const from = rateRef.current;
    const start = performance.now();
    let rafId = 0;

    const apply = (rate: number) => {
      ref.current?.getAnimations({ subtree: true }).forEach((animation) => {
        if (animation instanceof CSSAnimation && ORBIT_ANIMATIONS.has(animation.animationName)) {
          animation.playbackRate = rate;
        }
      });
    };

    const step = (now: number) => {
      const progress = Math.min(1, (now - start) / RAMP_MS);
      const eased = 1 - (1 - progress) ** 3;
      rateRef.current = from + (target - from) * eased;
      apply(rateRef.current);
      if (progress < 1) rafId = requestAnimationFrame(step);
    };

    rafId = requestAnimationFrame(step);
    return () => cancelAnimationFrame(rafId);
  }, [ref, moving, pinCount]);
}
