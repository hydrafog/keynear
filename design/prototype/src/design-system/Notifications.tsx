import type { ReactNode } from 'react';

export function NotificationCard({ children }: { children: ReactNode }) {
  return <article className="animate-kn-fade-in rounded-3xl p-4 kn-sys-notif">{children}</article>;
}

export function NotificationHeader({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <div className="flex min-w-0 items-center gap-2 text-kn-xs kn-text-muted">
      <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full kn-sys-qs-on">{icon}</span>
      <span className="min-w-0 truncate font-kn-semibold">{label}</span>
    </div>
  );
}
