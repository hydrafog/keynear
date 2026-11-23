import type { MouseEventHandler, ReactNode } from 'react';

const FOCUS_RING =
  'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus';

export function SelectionTile({
  icon,
  label,
  onClick,
}: {
  icon: ReactNode;
  label: string;
  onClick: MouseEventHandler<HTMLButtonElement>;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex min-h-(--kn-size-128px) cursor-pointer flex-col items-center justify-center gap-3 rounded-kn-2xl p-5 text-kn-base font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-97) kn-surface kn-surface-hover ${FOCUS_RING}`}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

const TONE_CLASS = {
  accent: 'kn-loading-bg',
  tonal: 'kn-surface-tonal kn-surface-hover',
  danger: 'kn-error-bg',
  'accent-tonal': 'kn-loading-tonal',
} as const;

export function ActionButton({
  tone,
  height,
  width,
  onClick,
  children,
}: {
  tone: keyof typeof TONE_CLASS;
  height: 46 | 48;
  width: 'grow' | 'block';
  onClick: MouseEventHandler<HTMLButtonElement>;
  children: ReactNode;
}) {
  const heightClass = height === 46 ? 'min-h-(--kn-size-46px)' : 'min-h-(--kn-size-48px)';
  const widthClass = width === 'grow' ? 'flex-1' : 'w-full';
  return (
    <button
      type="button"
      onClick={onClick}
      className={`${heightClass} ${widthClass} cursor-pointer rounded-kn-xl px-5 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-150) active:scale-(--kn-press-95) ${TONE_CLASS[tone]} ${FOCUS_RING}`}
    >
      {children}
    </button>
  );
}

export function RoundIconButton({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: MouseEventHandler<HTMLButtonElement>;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      className={`flex h-(--kn-size-44px) w-(--kn-size-44px) shrink-0 cursor-pointer items-center justify-center rounded-full transition-all duration-(--kn-duration-150) active:scale-(--kn-press-95) kn-surface-tonal kn-surface-hover ${FOCUS_RING}`}
    >
      {children}
    </button>
  );
}

export function ShadeTriggerButton({
  onClick,
  children,
}: {
  onClick: MouseEventHandler<HTMLButtonElement>;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`min-h-(--kn-size-44px) cursor-pointer rounded-full px-3.5 text-kn-xs font-kn-semibold transition-all duration-(--kn-duration-150) active:scale-(--kn-press-95) kn-proto-button ${FOCUS_RING}`}
    >
      {children}
    </button>
  );
}

/**
 * Compact system icon button. The button is a 44px hit area; `bg-clip-content` with
 * 6px padding keeps the visible circle at 32px, centered inside it.
 */
export function SystemIconButton({
  label,
  onClick,
  className = '',
  children,
}: {
  label: string;
  onClick: MouseEventHandler<HTMLButtonElement>;
  className?: string;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      className={`flex h-11 w-11 shrink-0 cursor-pointer items-center justify-center rounded-full p-1.5 bg-clip-content transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) kn-sys-notif-action ${className} ${FOCUS_RING}`}
    >
      {children}
    </button>
  );
}

export function QuickSettingsTile({
  className,
  ariaPressed,
  onClick,
  children,
}: {
  className: string;
  ariaPressed?: boolean;
  onClick: MouseEventHandler<HTMLButtonElement>;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={ariaPressed}
      className={`flex min-h-(--kn-size-68px) cursor-pointer items-center gap-3 rounded-full px-4 py-3 text-left transition-all duration-(--kn-duration-150) active:scale-(--kn-press-95) ${className} ${FOCUS_RING}`}
    >
      {children}
    </button>
  );
}
