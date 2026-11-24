import { ArrowUpRight, Download as DownloadIcon, Laptop, Smartphone } from 'lucide-react';
import { Reveal } from '../components/Reveal';
import { LINKS } from '../data/site';

export function Download() {
  return (
    <section id="download" className="mx-auto max-w-6xl px-4 py-16 sm:px-6 sm:py-24 lg:px-8">
      <div className="grid gap-10 lg:grid-cols-12 lg:items-start">
        <Reveal className="min-w-0 lg:col-span-4">
          <h2 className="kn-title">Download</h2>
          <p className="kn-body mt-3 max-w-[34ch] kn-text-secondary">
            Install the Android app on your phone and the companion daemon on Linux.
          </p>
        </Reveal>

        <div className="min-w-0 space-y-4 lg:col-span-8">
          <Reveal delay={60}>
            <div className="kn-surface kn-card-lift flex flex-col justify-between gap-4 rounded-2xl p-6 sm:flex-row sm:items-center">
              <div className="flex min-w-0 items-center gap-4">
                <span className="kn-accent-tonal inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-xl">
                  <Smartphone className="h-5 w-5" aria-hidden="true" />
                </span>
                <div className="min-w-0">
                  <h3 className="kn-heading">Android app</h3>
                  <p className="kn-caption mt-0.5 kn-text-muted">
                    Quick Settings tiles, Direct Share, and Android Keystore keys.
                  </p>
                </div>
              </div>

              <a
                href={LINKS.androidReleases}
                target="_blank"
                rel="noreferrer"
                className="kn-btn-primary kn-interactive kn-press inline-flex min-h-[44px] shrink-0 items-center justify-center gap-2 rounded-xl px-5 text-sm font-bold"
              >
                <DownloadIcon className="h-4 w-4" aria-hidden="true" />
                <span>Download APK</span>
              </a>
            </div>
          </Reveal>

          <Reveal delay={140}>
            <div className="kn-surface kn-card-lift flex flex-col justify-between gap-4 rounded-2xl p-6 sm:flex-row sm:items-center">
              <div className="flex min-w-0 items-center gap-4">
                <span className="kn-accent-tonal inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-xl">
                  <Laptop className="h-5 w-5" aria-hidden="true" />
                </span>
                <div className="min-w-0">
                  <h3 className="kn-heading">Linux daemon</h3>
                  <p className="kn-caption mt-0.5 kn-text-muted">
                    Bluetooth presence, Wi-Fi transfers, MPRIS2, and PipeWire audio.
                  </p>
                </div>
              </div>

              <a
                href={LINKS.linuxReleases}
                target="_blank"
                rel="noreferrer"
                className="kn-btn-tonal kn-interactive kn-press inline-flex min-h-[44px] shrink-0 items-center justify-center gap-1.5 rounded-xl px-5 text-sm font-bold"
              >
                <span>Linux packages</span>
                <ArrowUpRight className="h-4 w-4" aria-hidden="true" />
              </a>
            </div>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
