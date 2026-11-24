import { Moon, Sun } from 'lucide-react';
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { KeynearIcon } from '../components/KeynearIcon';
import { SLIDES } from '../data/site';

export interface HeaderProps {
  darkMode: boolean;
  activeSlide: number;
  onToggleTheme: () => void;
  onSelectSlide: (index: number) => void;
}

export function Header({ darkMode, activeSlide, onToggleTheme, onSelectSlide }: HeaderProps) {
  const trackRef = useRef<HTMLDivElement | null>(null);
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const [indicator, setIndicator] = useState({ x: 0, width: 0, ready: false });

  const measure = useCallback(() => {
    const activeTab = tabRefs.current[activeSlide];
    if (!activeTab) return;
    setIndicator({
      x: activeTab.offsetLeft,
      width: activeTab.offsetWidth,
      ready: true,
    });
  }, [activeSlide]);

  useLayoutEffect(() => {
    measure();
  }, [measure]);

  useEffect(() => {
    const track = trackRef.current;
    if (!track || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(() => measure());
    observer.observe(track);
    return () => observer.disconnect();
  }, [measure]);

  return (
    <header className="kn-nav-bar relative shrink-0">
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-3 px-4 sm:px-6 lg:px-8">
        <a
          href="#main-content"
          onClick={(e) => {
            e.preventDefault();
            onSelectSlide(0);
          }}
          className="kn-interactive group flex shrink-0 items-center gap-2.5 rounded-xl py-1 pr-2 text-lg font-extrabold tracking-tight"
        >
          <KeynearIcon size={32} className="transition-transform duration-300 group-hover:scale-105" />
          <span>Keynear</span>
        </a>

        <div className="flex items-center gap-2.5">
          <button
            type="button"
            onClick={onToggleTheme}
            aria-label={darkMode ? 'Switch to light theme' : 'Switch to dark theme'}
            className="kn-theme-btn kn-btn-tonal kn-interactive kn-press flex h-11 w-11 shrink-0 cursor-pointer items-center justify-center rounded-xl"
          >
            {darkMode ? (
              <Sun className="kn-theme-icon h-4 w-4" aria-hidden="true" />
            ) : (
              <Moon className="kn-theme-icon h-4 w-4" aria-hidden="true" />
            )}
          </button>

          <div
            ref={trackRef}
            role="tablist"
            aria-label="Page sections"
            className="kn-surface relative inline-flex items-center rounded-full p-1"
          >
            <span
              aria-hidden="true"
              className="kn-seg-indicator kn-accent-tonal pointer-events-none absolute bottom-1 top-1 left-0 rounded-full"
              style={{
                width: `${indicator.width}px`,
                transform: `translate3d(${indicator.x}px, 0, 0)`,
                opacity: indicator.ready ? 1 : 0,
              }}
            />
            {SLIDES.map((slide, idx) => {
              const active = activeSlide === slide.index;
              return (
                <button
                  key={slide.id}
                  ref={(node) => {
                    tabRefs.current[idx] = node;
                  }}
                  type="button"
                  role="tab"
                  aria-selected={active}
                  onClick={() => onSelectSlide(slide.index)}
                  className={`kn-interactive relative z-10 inline-flex min-h-[36px] cursor-pointer items-center gap-1.5 rounded-full px-3 text-xs font-bold transition-colors sm:px-3.5 ${
                    active ? 'kn-seg-active-text' : 'kn-text-muted hover:kn-text-primary'
                  }`}
                >
                  <span className="font-mono tabular-nums">{slide.step}</span>
                  <span className="hidden sm:inline">{slide.label}</span>
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </header>
  );
}
