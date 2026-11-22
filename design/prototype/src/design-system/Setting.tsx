import type { ReactNode } from 'react';

export function SectionHeading({ children }: { children: ReactNode }) {
  return <h2 className="mb-2.5 px-1 text-kn-xs font-kn-bold uppercase tracking-kn-wider kn-text-muted">{children}</h2>;
}

export function SettingsSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <SectionHeading>{title}</SectionHeading>
      <div className="space-y-1.5 rounded-kn-2xl p-2 kn-surface">{children}</div>
    </section>
  );
}

export function SettingRow({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="flex min-h-(--kn-size-66px) items-center justify-between gap-3 rounded-kn-xl px-4 py-3">
      <div className="min-w-0 flex-1 pr-2">
        <div className="break-words text-kn-base font-kn-semibold">{title}</div>
        <div className="mt-0.5 break-words text-kn-xs kn-text-muted">{description}</div>
      </div>
      {children}
    </div>
  );
}
