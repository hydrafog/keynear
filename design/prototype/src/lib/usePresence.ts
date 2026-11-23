import { useEffect, useState } from 'react';

export interface Presence<T> {
  /** True while the element should be in the DOM, including during its exit animation. */
  mounted: boolean;
  /** True once `value` went falsy and the exit animation is playing. */
  closing: boolean;
  /** The latest truthy value, retained so the exit animation can still render it. */
  value: T | null;
}

/** Keeps an element mounted for `exitMs` after `value` turns falsy so it can animate out. */
export function usePresence<T>(value: T | null | false | undefined, exitMs: number): Presence<T> {
  const open = Boolean(value);
  const [retained, setRetained] = useState<{ mounted: boolean; value: T | null }>({
    mounted: open,
    value: open ? (value as T) : null,
  });

  useEffect(() => {
    if (open) {
      setRetained({ mounted: true, value: value as T });
      return;
    }
    const timer = window.setTimeout(() => setRetained((prev) => ({ ...prev, mounted: false })), exitMs);
    return () => window.clearTimeout(timer);
  }, [open, value, exitMs]);

  return {
    mounted: open || retained.mounted,
    closing: !open,
    value: open ? (value as T) : retained.value,
  };
}
