import { KeynearIcon } from '../components/KeynearIcon';
import { LINKS, SLIDES } from '../data/site';

export interface FooterProps {
  activeSlide: number;
  onSelectSlide: (index: number) => void;
}

export function Footer({ activeSlide, onSelectSlide }: FooterProps) {
  return (
    <footer className="relative z-20 shrink-0">
      <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4 px-4 py-4 sm:px-6 lg:px-8">
        <div className="flex items-center gap-2.5 text-sm font-bold">
          <KeynearIcon size={24} />
          <span>Keynear</span>
        </div>

        <div
          role="tablist"
          aria-label="Page sections"
          className="kn-surface inline-flex items-center gap-1 rounded-full p-1"
        >
          {SLIDES.map((slide) => {
            const active = activeSlide === slide.index;
            return (
              <button
                key={slide.id}
                type="button"
                role="tab"
                aria-selected={active}
                onClick={() => onSelectSlide(slide.index)}
                className={`kn-interactive inline-flex min-h-[34px] cursor-pointer items-center gap-1.5 rounded-full px-3.5 text-xs font-bold transition-colors ${
                  active ? 'kn-accent-tonal' : 'kn-text-muted hover:kn-text-primary'
                }`}
              >
                <span className="font-mono tabular-nums">{slide.step}</span>
                <span>{slide.label}</span>
              </button>
            );
          })}
        </div>

        <div className="flex flex-wrap items-center gap-5 text-sm font-semibold kn-text-muted">
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
