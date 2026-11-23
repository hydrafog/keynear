import type { VisibilityMode } from '../types';

export const VISIBILITY_LABELS: Record<VisibilityMode, { full: string; short: string }> = {
  hidden: { full: 'Hidden', short: 'Hidden' },
  pairedOnly: { full: 'Paired only', short: 'Paired' },
  everyoneNearby: { full: 'Everyone nearby', short: 'Everyone' },
};

export function pluralize(count: number, noun: string, plural = `${noun}s`): string {
  return `${count} ${count === 1 ? noun : plural}`;
}

export function joinMeta(...parts: Array<string | number | null | undefined | false>): string {
  return parts.filter((part) => part !== null && part !== undefined && part !== false && part !== '').join(' · ');
}
