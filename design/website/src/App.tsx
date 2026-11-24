import { useCallback, useEffect, useRef, useState } from 'react';
import type { ThemeMode } from './data/site';
import { SLIDES } from './data/site';
import { Footer } from './sections/Footer';
import { Header } from './sections/Header';
import { Hero } from './sections/Hero';
import { HowItWorks } from './sections/HowItWorks';
import { Privacy } from './sections/Privacy';

const WHEEL_COOLDOWN_MS = 540;
const WHEEL_THRESHOLD = 18;
const SWIPE_THRESHOLD = 44;

interface TouchOrigin {
  x: number;
  y: number;
  atTop: boolean;
  atBottom: boolean;
}

export default function App() {
  const [darkMode, setDarkMode] = useState(false);
  const [activeSlide, setActiveSlide] = useState(0);
  const lastWheelRef = useRef(0);
  const touchStartRef = useRef<TouchOrigin | null>(null);
  const panelRefs = useRef<(HTMLDivElement | null)[]>([]);

  useEffect(() => {
    try {
      if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
        if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
          setDarkMode(true);
        }
      }
    } catch {
      setDarkMode(false);
    }
  }, []);

  const selectSlide = useCallback((nextIndex: number) => {
    const clamped = Math.max(0, Math.min(SLIDES.length - 1, nextIndex));
    setActiveSlide(clamped);
    const targetPanel = panelRefs.current[clamped];
    if (targetPanel) {
      targetPanel.scrollTop = 0;
    }
  }, []);

  const stepSlide = useCallback(
    (direction: 1 | -1) => {
      setActiveSlide((current) => {
        const next = Math.max(0, Math.min(SLIDES.length - 1, current + direction));
        if (next !== current) {
          const targetPanel = panelRefs.current[next];
          if (targetPanel) targetPanel.scrollTop = 0;
        }
        return next;
      });
    },
    []
  );

  useEffect(() => {
    const onWheel = (event: WheelEvent) => {
      const primaryDelta =
        Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
      if (Math.abs(primaryDelta) < WHEEL_THRESHOLD) return;

      const activePanel = panelRefs.current[activeSlide];
      if (activePanel && Math.abs(event.deltaY) >= Math.abs(event.deltaX)) {
        const canScrollDown =
          activePanel.scrollTop + activePanel.clientHeight < activePanel.scrollHeight - 2;
        const canScrollUp = activePanel.scrollTop > 2;
        if ((event.deltaY > 0 && canScrollDown) || (event.deltaY < 0 && canScrollUp)) {
          return;
        }
      }

      event.preventDefault();
      const now = Date.now();
      if (now - lastWheelRef.current < WHEEL_COOLDOWN_MS) return;
      lastWheelRef.current = now;
      stepSlide(primaryDelta > 0 ? 1 : -1);
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'ArrowRight' || event.key === 'PageDown') {
        stepSlide(1);
      } else if (event.key === 'ArrowLeft' || event.key === 'PageUp') {
        stepSlide(-1);
      }
    };

    window.addEventListener('wheel', onWheel, { passive: false });
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('wheel', onWheel);
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [activeSlide, stepSlide]);

  const theme: ThemeMode = darkMode ? 'dark' : 'light';

  const panelPosition = (index: number): 'prev' | 'active' | 'next' => {
    if (index === activeSlide) return 'active';
    return index < activeSlide ? 'prev' : 'next';
  };

  return (
    <div
      data-kn-theme={theme}
      className="kn-root flex h-[100svh] flex-col overflow-hidden antialiased"
      onTouchStart={(event) => {
        const touch = event.touches[0];
        if (!touch) return;
        const panel = panelRefs.current[activeSlide];
        const atTop = !panel || panel.scrollTop <= 2;
        const atBottom =
          !panel || panel.scrollTop + panel.clientHeight >= panel.scrollHeight - 2;
        touchStartRef.current = {
          x: touch.clientX,
          y: touch.clientY,
          atTop,
          atBottom,
        };
      }}
      onTouchEnd={(event) => {
        const start = touchStartRef.current;
        const touch = event.changedTouches[0];
        touchStartRef.current = null;
        if (!start || !touch) return;

        const now = Date.now();
        if (now - lastWheelRef.current < WHEEL_COOLDOWN_MS) return;

        const dx = touch.clientX - start.x;
        const dy = touch.clientY - start.y;

        if (Math.abs(dy) >= Math.abs(dx) && Math.abs(dy) > SWIPE_THRESHOLD) {
          const panel = panelRefs.current[activeSlide];
          const nowAtTop = !panel || panel.scrollTop <= 2;
          const nowAtBottom =
            !panel || panel.scrollTop + panel.clientHeight >= panel.scrollHeight - 2;

          if (dy < 0 && start.atBottom && nowAtBottom) {
            lastWheelRef.current = now;
            stepSlide(1);
          } else if (dy > 0 && start.atTop && nowAtTop) {
            lastWheelRef.current = now;
            stepSlide(-1);
          }
          return;
        }

        if (Math.abs(dx) > SWIPE_THRESHOLD && Math.abs(dx) > Math.abs(dy) * 1.2) {
          lastWheelRef.current = now;
          stepSlide(dx < 0 ? 1 : -1);
        }
      }}
    >
      <a
        href="#main-content"
        className="kn-skip-link kn-btn-primary kn-interactive rounded-xl px-4 py-2.5 text-sm font-bold"
      >
        Skip to content
      </a>

      <Header
        darkMode={darkMode}
        activeSlide={activeSlide}
        onToggleTheme={() => setDarkMode((current) => !current)}
        onSelectSlide={selectSlide}
      />

      <main id="main-content" className="kn-wheel-stage relative flex-1 overflow-hidden">
        <div
          ref={(node) => {
            panelRefs.current[0] = node;
          }}
          data-position={panelPosition(0)}
          aria-hidden={activeSlide !== 0}
          className="kn-wheel-panel"
        >
          <Hero theme={theme} />
        </div>

        <div
          ref={(node) => {
            panelRefs.current[1] = node;
          }}
          data-position={panelPosition(1)}
          aria-hidden={activeSlide !== 1}
          className="kn-wheel-panel"
        >
          <HowItWorks />
        </div>

        <div
          ref={(node) => {
            panelRefs.current[2] = node;
          }}
          data-position={panelPosition(2)}
          aria-hidden={activeSlide !== 2}
          className="kn-wheel-panel"
        >
          <Privacy />
        </div>
      </main>

      <Footer />
    </div>
  );
}
