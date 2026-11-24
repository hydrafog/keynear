import { WireframeSlot } from '../components/WireframeSlot';
import { CAPABILITIES, PAIRING_STEPS } from '../data/site';

export function HowItWorks() {
  return (
    <section
      id="how-it-works"
      className="mx-auto flex min-h-full w-full max-w-6xl items-center px-4 py-6 sm:px-6 lg:px-8"
    >
      <div className="grid w-full gap-8 lg:grid-cols-12 lg:items-center">
        <div className="min-w-0 lg:col-span-4">
          <h2 className="kn-title">How it works</h2>
          <p className="kn-desc mt-2 max-w-[30ch] kn-text-secondary">
            No accounts or cloud relays.
          </p>

          <ol className="mt-6 space-y-4">
            {PAIRING_STEPS.map((item) => (
              <li key={item.step} className="flex items-start gap-3.5">
                <span
                  aria-hidden="true"
                  className="kn-surface font-mono inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-xl text-xs font-bold tabular-nums kn-accent-text"
                >
                  {item.step}
                </span>

                <div className="min-w-0 flex-1">
                  <h3 className="kn-heading">{item.title}</h3>
                  <p className="kn-caption mt-1 kn-text-secondary">{item.detail}</p>
                </div>
              </li>
            ))}
          </ol>
        </div>

        <div className="min-w-0 lg:col-span-8">
          <div className="kn-wireframe-grid">
            {CAPABILITIES.map(({ id, src, alt, title, detail }) => (
              <article key={id} className="kn-wireframe-cell">
                <WireframeSlot src={src} alt={alt} />
                <div>
                  <h4 className="kn-heading">{title}</h4>
                  <p className="kn-caption mt-1 kn-text-secondary">{detail}</p>
                </div>
              </article>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
