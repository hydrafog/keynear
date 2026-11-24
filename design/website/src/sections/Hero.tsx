import type { CSSProperties } from 'react';
import { Download, Smartphone } from 'lucide-react';
import { ProximityHalo } from '../components/ProximityHalo';
import { ScreenshotFrame } from '../components/ScreenshotFrame';
import type { ThemeMode } from '../data/site';
import { HERO_SCREENSHOT, LINKS } from '../data/site';

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

export interface HeroProps {
  theme: ThemeMode;
  onDownload: () => void;
}

export function Hero({ theme, onDownload }: HeroProps) {
  return (
    <section className="mx-auto grid max-w-6xl items-center gap-12 px-4 pb-16 pt-10 sm:px-6 sm:pt-16 lg:grid-cols-12 lg:gap-10 lg:px-8 lg:pb-24 lg:pt-20">
      <div className="min-w-0 lg:col-span-7">
        <h1 className="kn-display kn-hero-in" style={delayStyle(40)}>
          Android and Linux, right next door.
        </h1>

        <p
          className="kn-body kn-hero-in mt-6 max-w-[52ch] kn-text-secondary"
          style={delayStyle(140)}
        >
          Send files, sync your clipboard, mirror notifications, and control desktop media
          directly over Bluetooth and local Wi-Fi.
        </p>

        <div
          className="kn-hero-in mt-9 flex flex-wrap items-center gap-3.5"
          style={delayStyle(240)}
        >
          <button
            type="button"
            onClick={onDownload}
            className="kn-btn-primary kn-interactive kn-press inline-flex min-h-[48px] cursor-pointer items-center gap-2 rounded-xl px-6 text-sm font-bold"
          >
            <Download className="h-4 w-4" aria-hidden="true" />
            <span>Download Keynear</span>
          </button>

          <a
            href={LINKS.github}
            target="_blank"
            rel="noreferrer"
            className="kn-btn-tonal kn-interactive kn-press inline-flex min-h-[48px] items-center gap-2 rounded-xl px-6 text-sm font-bold"
          >
            <GitHubMark />
            <span>Read source code</span>
          </a>
        </div>
      </div>

      <figure
        className="kn-hero-in relative mx-auto w-full max-w-[280px] py-4 lg:col-span-5"
        style={delayStyle(180)}
      >
        <ProximityHalo />
        <div className="relative">
          <ScreenshotFrame
            src={HERO_SCREENSHOT.src[theme]}
            alt={HERO_SCREENSHOT.alt}
            label={HERO_SCREENSHOT.label}
            icon={Smartphone}
            float
          />
        </div>
      </figure>
    </section>
  );
}
