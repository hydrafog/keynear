import { Download, Menu, Moon, Sun, X } from 'lucide-react';
import { useState } from 'react';
import { KeynearIcon } from '../components/KeynearIcon';
import { LINKS, SLIDES } from '../data/site';

export interface HeaderProps {
  darkMode: boolean;
  activeSlide: number;
  onToggleTheme: () => void;
  onSelectSlide: (index: number) => void;
}

export function Header({ darkMode, activeSlide, onToggleTheme, onSelectSlide }: HeaderProps) {
  const [menuOpen, setMenuOpen] = useState(false);

  const handleSelect = (index: number) => {
    setMenuOpen(false);
    onSelectSlide(index);
  };

  return (
    <header className="kn-nav-bar relative shrink-0">
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-4 px-4 sm:px-6 lg:px-8">
        <a
          href="#main-content"
          onClick={(e) => {
            e.preventDefault();
            handleSelect(0);
          }}
          className="kn-interactive group flex items-center gap-2.5 rounded-xl py-1 pr-2 text-lg font-extrabold tracking-tight"
        >
          <KeynearIcon size={32} className="transition-transform duration-300 group-hover:scale-105" />
          <span>Keynear</span>
        </a>

        <nav aria-label="Primary" className="hidden items-center gap-5 md:flex">
          {SLIDES.map((slide) => {
            const active = activeSlide === slide.index;
            return (
              <button
                key={slide.id}
                type="button"
                aria-current={active ? 'page' : undefined}
                data-active={active}
                onClick={() => handleSelect(slide.index)}
                className={`kn-nav-link kn-interactive cursor-pointer rounded-lg px-2 py-2 text-sm font-semibold transition-colors ${
                  active ? 'kn-text-primary' : 'kn-text-secondary hover:kn-text-primary'
                }`}
              >
                {slide.label}
              </button>
            );
          })}
        </nav>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onToggleTheme}
            aria-label={darkMode ? 'Switch to light theme' : 'Switch to dark theme'}
            className="kn-theme-btn kn-btn-tonal kn-interactive kn-press flex h-11 w-11 cursor-pointer items-center justify-center rounded-xl"
          >
            {darkMode ? (
              <Sun className="kn-theme-icon h-4 w-4" aria-hidden="true" />
            ) : (
              <Moon className="kn-theme-icon h-4 w-4" aria-hidden="true" />
            )}
          </button>

          <a
            href={LINKS.releases}
            target="_blank"
            rel="noreferrer"
            className="kn-btn-primary kn-interactive kn-press inline-flex min-h-[44px] cursor-pointer items-center gap-2 rounded-xl px-4 text-sm font-bold"
          >
            <Download className="h-4 w-4" aria-hidden="true" />
            <span>Download</span>
          </a>

          <button
            type="button"
            onClick={() => setMenuOpen((open) => !open)}
            aria-label="Open navigation menu"
            aria-expanded={menuOpen}
            className="kn-btn-tonal kn-interactive kn-press flex h-11 w-11 cursor-pointer items-center justify-center rounded-xl md:hidden"
          >
            {menuOpen ? (
              <X className="h-5 w-5" aria-hidden="true" />
            ) : (
              <Menu className="h-5 w-5" aria-hidden="true" />
            )}
          </button>
        </div>
      </div>

      {menuOpen && (
        <nav
          aria-label="Mobile"
          className="absolute inset-x-0 top-16 z-30 mx-auto max-w-6xl px-4 pb-4 pt-2 md:hidden"
        >
          <div className="kn-surface flex flex-col gap-1 rounded-2xl p-2">
            {SLIDES.map((slide) => (
              <button
                key={slide.id}
                type="button"
                aria-current={activeSlide === slide.index ? 'page' : undefined}
                onClick={() => handleSelect(slide.index)}
                className="kn-interactive min-h-[44px] cursor-pointer rounded-xl px-3.5 py-2.5 text-left text-sm font-semibold kn-text-secondary hover:kn-text-primary"
              >
                {slide.label}
              </button>
            ))}
          </div>
        </nav>
      )}
    </header>
  );
}
