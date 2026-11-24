import type { CSSProperties } from 'react';
import { useState } from 'react';
import { Download } from 'lucide-react';
import { ProximityHalo } from '../components/ProximityHalo';
import { ScreenshotFrame } from '../components/ScreenshotFrame';
import type { ThemeMode } from '../data/site';
import { LINKS, SCREENSHOTS } from '../data/site';

function GitHubMark() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      className="h-4 w-4"
      aria-hidden="true"
    >
      <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
      <path d="M9 18c-4.51 2-5-2-7-2" />
    </svg>
  );
}

type KnDelayStyle = CSSProperties & {
  '--kn-delay': string;
};

function delayStyle(ms: number): KnDelayStyle {
  return { '--kn-delay': `${ms}ms` };
}

type StackSlot = 'front' | 'right' | 'left';

function slotFor(index: number, activeIndex: number, total: number): StackSlot {
  const offset = (index - activeIndex + total) % total;
  if (offset === 0) return 'front';
  if (offset === 1) return 'right';
  return 'left';
}

export interface HeroProps {
  theme: ThemeMode;
}

export function Hero({ theme }: HeroProps) {
  const [activeIndex, setActiveIndex] = useState(0);

  return (
    <section className="mx-auto flex min-h-full w-full max-w-6xl items-center px-6 pb-10 pt-3 sm:px-6 sm:py-6 lg:px-8">
      <div className="my-auto grid w-full items-center gap-5 sm:gap-8 lg:grid-cols-12 lg:gap-10">
        <div className="min-w-0 lg:col-span-7">
          <h1 className="kn-display kn-hero-in" style={delayStyle(40)}>
            Android and Linux, right next door.
          </h1>

          <p
            className="kn-body kn-hero-in mt-3 max-w-[52ch] kn-text-secondary sm:mt-5"
            style={delayStyle(140)}
          >
            Send files, sync your clipboard, mirror notifications, and control desktop media
            directly over Bluetooth and local Wi-Fi.
          </p>

          <div
            className="kn-hero-in mt-5 flex flex-wrap items-center gap-2.5 sm:mt-8 sm:gap-3.5"
            style={delayStyle(240)}
          >
            <a
              href={LINKS.releases}
              target="_blank"
              rel="noreferrer"
              className="kn-btn-primary kn-interactive kn-press inline-flex min-h-[44px] items-center gap-2 rounded-xl px-4 text-sm font-bold sm:min-h-[48px] sm:px-6"
            >
              <Download className="h-4 w-4" aria-hidden="true" />
              <span>Download Keynear</span>
            </a>

            <a
              href={LINKS.github}
              target="_blank"
              rel="noreferrer"
              className="kn-btn-tonal kn-interactive kn-press inline-flex min-h-[44px] items-center gap-2 rounded-xl px-4 text-sm font-bold sm:min-h-[48px] sm:px-6"
            >
              <GitHubMark />
              <span>Read source code</span>
            </a>
          </div>
        </div>

        <figure
          className="kn-hero-in relative mx-auto w-full max-w-[162px] pb-7 pt-2 sm:max-w-[232px] sm:py-4 lg:col-span-5 lg:max-w-[248px]"
          style={delayStyle(180)}
        >
          <ProximityHalo />
          <div className="kn-hero-stack relative w-full">
            {SCREENSHOTS.map((shot, idx) => {
              const slot = slotFor(idx, activeIndex, SCREENSHOTS.length);
              const isFront = slot === 'front';
              return (
                <button
                  key={shot.id}
                  type="button"
                  data-slot={slot}
                  aria-label={isFront ? `Keynear ${shot.label}` : `Show ${shot.label}`}
                  aria-pressed={isFront}
                  onClick={() => setActiveIndex(idx)}
                  className="kn-stack-card kn-interactive block w-full rounded-3xl p-0 text-left"
                >
                  <ScreenshotFrame
                    src={shot.src[theme]}
                    alt={`Keynear ${shot.label}`}
                    label={shot.label}
                    icon={shot.icon}
                  />
                </button>
              );
            })}
          </div>
        </figure>
      </div>
    </section>
  );
}
