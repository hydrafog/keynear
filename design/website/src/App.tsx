import { useEffect, useState } from 'react';
import type { ThemeMode } from './data/site';
import { Capabilities } from './sections/Capabilities';
import { Download } from './sections/Download';
import { Footer } from './sections/Footer';
import { Header } from './sections/Header';
import { Hero } from './sections/Hero';
import { HowItWorks } from './sections/HowItWorks';

export default function App() {
  const [darkMode, setDarkMode] = useState(false);

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

  const theme: ThemeMode = darkMode ? 'dark' : 'light';

  const scrollTo = (id: string) => {
    document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  };

  return (
    <div data-kn-theme={theme} className="kn-root min-h-[100svh] antialiased">
      <a
        href="#main-content"
        className="kn-skip-link kn-btn-primary kn-interactive rounded-xl px-4 py-2.5 text-sm font-bold"
      >
        Skip to content
      </a>

      <Header
        darkMode={darkMode}
        onToggleTheme={() => setDarkMode((current) => !current)}
        onNavigate={scrollTo}
      />

      <main id="main-content">
        <Hero theme={theme} onDownload={() => scrollTo('download')} />
        <HowItWorks />
        <Capabilities />
        <Download />
      </main>

      <Footer />
    </div>
  );
}
