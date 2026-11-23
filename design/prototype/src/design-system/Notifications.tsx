import type { ReactNode } from 'react';
import { KeynearMark } from './KeynearMark';

export function NotificationCard({ children }: { children: ReactNode }) {
  return <article className="animate-kn-fade-in rounded-kn-3xl p-4 kn-sys-notif">{children}</article>;
}

/** App badge for every Keynear notification: the symbolic mark on a flat system-accent circle. */
export function NotificationBadge() {
  return (
    <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full kn-sys-accent">
      <KeynearMark size={12} variant="symbolic" />
    </span>
  );
}

export function NotificationHeader({ label }: { label: string }) {
  return (
    <div className="flex min-w-0 items-center gap-2 text-kn-xs kn-text-muted">
      <NotificationBadge />
      <span className="min-w-0 truncate font-kn-semibold">{label}</span>
    </div>
  );
}
