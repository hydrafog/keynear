import { Reveal } from '../components/Reveal';
import { PAIRING_STEPS } from '../data/site';

export function HowItWorks() {
  return (
    <section id="how-it-works" className="mx-auto max-w-6xl px-4 py-16 sm:px-6 sm:py-24 lg:px-8">
      <div className="grid gap-10 lg:grid-cols-12 lg:items-start">
        <Reveal className="min-w-0 lg:col-span-4">
          <h2 className="kn-title">How it works</h2>
          <p className="kn-body mt-3 max-w-[34ch] kn-text-secondary">
            No accounts or cloud relays. Keys are verified in person and stay on your hardware.
          </p>
        </Reveal>

        <ol className="min-w-0 space-y-6 lg:col-span-8">
          {PAIRING_STEPS.map((item, idx) => (
            <Reveal key={item.step} as="li" delay={idx * 90} className="flex items-start gap-4 sm:gap-5">
              <span
                aria-hidden="true"
                className="kn-surface font-mono inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-xl text-sm font-bold tabular-nums kn-accent-text"
              >
                {item.step}
              </span>

              <div className="min-w-0 flex-1 pt-0.5">
                <h3 className="kn-heading">{item.title}</h3>
                <p className="kn-desc mt-1.5 max-w-[56ch] kn-text-secondary">{item.detail}</p>
              </div>
            </Reveal>
          ))}
        </ol>
      </div>
    </section>
  );
}
