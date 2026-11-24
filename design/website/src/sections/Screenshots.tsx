import { Reveal } from '../components/Reveal';
import { ScreenshotFrame } from '../components/ScreenshotFrame';
import type { ThemeMode } from '../data/site';
import { SCREENSHOTS } from '../data/site';

export interface ScreenshotsProps {
  theme: ThemeMode;
}

export function Screenshots({ theme }: ScreenshotsProps) {
  return (
    <section id="screenshots" className="mx-auto max-w-6xl px-4 py-16 sm:px-6 sm:py-24 lg:px-8">
      <Reveal className="max-w-xl">
        <h2 className="kn-title">Built for Android and Linux</h2>
        <p className="kn-body mt-3 kn-text-secondary">
          Native Quick Settings tiles, Direct Share targets, and Live Update transfer
          notifications.
        </p>
      </Reveal>

      <div className="kn-shots-grid mt-12">
        {SCREENSHOTS.map((shot, idx) => (
          <Reveal key={shot.id} as="figure" delay={idx * 100} className="mx-auto w-full max-w-[280px]">
            <ScreenshotFrame
              src={shot.src[theme]}
              alt={`Keynear ${shot.label}`}
              label={shot.label}
              icon={shot.icon}
            />
            <figcaption className="mt-4 px-1">
              <div className="text-sm font-bold">{shot.label}</div>
              <p className="kn-caption mt-1 kn-text-muted">{shot.caption}</p>
            </figcaption>
          </Reveal>
        ))}
      </div>
    </section>
  );
}
