import { Reveal } from '../components/Reveal';
import { WireframeSlot } from '../components/WireframeSlot';
import { CAPABILITIES } from '../data/site';

export function Capabilities() {
  return (
    <section id="capabilities" className="mx-auto max-w-6xl px-4 py-16 sm:px-6 sm:py-24 lg:px-8">
      <Reveal>
        <h2 className="kn-title">Everything stays on your Wi-Fi</h2>
      </Reveal>

      <Reveal delay={80} className="mt-10">
        <div className="kn-wireframe-grid">
          {CAPABILITIES.map(({ id, src, alt, title, detail }) => (
            <article key={id} className="kn-wireframe-cell">
              <WireframeSlot src={src} alt={alt} />
              <div>
                <h3 className="kn-heading">{title}</h3>
                <p className="kn-desc mt-1.5 kn-text-secondary">{detail}</p>
              </div>
            </article>
          ))}
        </div>
      </Reveal>
    </section>
  );
}
