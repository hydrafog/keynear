import { KeyRound, Lock, ShieldCheck, WifiOff } from 'lucide-react';
import { LINKS, PRIVACY_ITEMS } from '../data/site';

const PRIVACY_ICONS = {
  telemetry: WifiOff,
  keys: KeyRound,
  memory: Lock,
  permissions: ShieldCheck,
} as const;

export function Privacy() {
  return (
    <section
      id="privacy"
      className="mx-auto flex min-h-full w-full max-w-6xl px-6 pb-8 pt-4 sm:px-6 sm:py-6 lg:px-8"
    >
      <div className="my-auto grid w-full gap-5 sm:gap-7 lg:grid-cols-12 lg:items-center lg:gap-8">
        <div className="min-w-0 lg:col-span-4">
          <h2 className="kn-title">Privacy policy</h2>
          <p className="kn-desc mt-2 max-w-[30ch] kn-text-secondary">
            No telemetry, accounts, or servers.
          </p>
          <p className="kn-caption mt-3 kn-text-muted">
            GPLv3 ·{' '}
            <a
              href={`${LINKS.github}/blob/main/docs/security-compliance/permissions-and-privacy.md`}
              target="_blank"
              rel="noreferrer"
              className="kn-interactive kn-accent-text underline underline-offset-4"
            >
              Permissions spec
            </a>
          </p>
        </div>

        <div className="min-w-0 lg:col-span-8">
          <div className="kn-wireframe-grid">
            {PRIVACY_ITEMS.map(({ id, title, detail }) => {
              const Icon = PRIVACY_ICONS[id];
              return (
                <article key={id} className="kn-wireframe-cell gap-4">
                  <span className="kn-accent-tonal inline-flex h-11 w-11 items-center justify-center rounded-xl">
                    <Icon className="h-5 w-5" aria-hidden="true" />
                  </span>
                  <div>
                    <h3 className="kn-heading">{title}</h3>
                    <p className="kn-caption mt-1.5 kn-text-secondary">{detail}</p>
                  </div>
                </article>
              );
            })}
          </div>
        </div>
      </div>
    </section>
  );
}

