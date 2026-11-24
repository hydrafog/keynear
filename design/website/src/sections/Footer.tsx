import { LINKS } from '../data/site';

export function Footer() {
  return (
    <footer className="relative z-20 shrink-0 pb-[env(safe-area-inset-bottom)]">
      <div className="mx-auto flex max-w-6xl items-center justify-center px-6 py-3 sm:px-6 sm:py-4 lg:px-8">
        <div className="flex flex-wrap items-center justify-center gap-6 text-sm font-semibold kn-text-muted">
          <a
            href={LINKS.github}
            target="_blank"
            rel="noreferrer"
            className="kn-interactive rounded-lg py-1 underline-offset-4 transition-colors hover:underline hover:kn-text-primary"
          >
            GitHub repository
          </a>
          <a
            href={LINKS.issues}
            target="_blank"
            rel="noreferrer"
            className="kn-interactive rounded-lg py-1 underline-offset-4 transition-colors hover:underline hover:kn-text-primary"
          >
            Report an issue
          </a>
        </div>
      </div>
    </footer>
  );
}
