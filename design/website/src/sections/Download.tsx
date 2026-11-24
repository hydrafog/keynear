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

        <div className="min-w-0 lg:col-span-8">
          <Reveal delay={60}>
            <div className="kn-surface kn-card-lift flex flex-col gap-6 rounded-2xl p-6 sm:p-7">
              <div className="flex flex-col gap-5">
                <div className="flex items-start gap-4">
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

                <div className="flex items-start gap-4">
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
              </div>

              <div>
                <a
                  href={LINKS.releases}
                  target="_blank"
                  rel="noreferrer"
                  className="kn-btn-primary kn-interactive kn-press inline-flex min-h-[46px] w-full items-center justify-center gap-2 rounded-xl px-5 text-sm font-bold sm:w-auto"
                >
                  <DownloadIcon className="h-4 w-4" aria-hidden="true" />
                  <span>Download from GitHub Releases</span>
                  <ArrowUpRight className="h-4 w-4" aria-hidden="true" />
                </a>
              </div>
            </div>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
