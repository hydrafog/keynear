import { KeynearIcon } from '../components/KeynearIcon';
import { LINKS } from '../data/site';

export function Footer() {
  return (
    <footer className="mx-auto max-w-6xl px-4 pb-12 pt-8 sm:px-6 lg:px-8">
      <div className="flex flex-col items-start justify-between gap-4 sm:flex-row sm:items-center">
        <div className="flex items-center gap-2.5 text-sm font-bold">
          <KeynearIcon size={24} />
          <span>Keynear</span>
        </div>

        <div className="flex flex-wrap items-center gap-6 text-sm font-semibold kn-text-muted">
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
