import type { LucideIcon } from 'lucide-react';
import { Smartphone } from 'lucide-react';
import { useEffect, useState } from 'react';

export interface ScreenshotFrameProps {
  src: string;
  alt: string;
  label: string;
  icon?: LucideIcon;
  float?: boolean;
}

export function ScreenshotFrame({
  src,
  alt,
  label,
  icon: Icon = Smartphone,
  float = false,
}: ScreenshotFrameProps) {
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setFailed(false);
  }, [src]);

  const showImage = Boolean(src) && !failed;

  return (
    <div
      className={`kn-frame kn-card-lift relative w-full overflow-hidden rounded-3xl ${
        float ? 'kn-hero-float' : ''
      }`}
      style={{ aspectRatio: '9 / 19' }}
    >
      {showImage ? (
        <img
          src={src}
          alt={alt}
          onError={() => setFailed(true)}
          className="h-full w-full object-cover"
        />
      ) : (
        <div
          role="img"
          aria-label={alt}
          className="kn-placeholder flex h-full w-full flex-col items-center justify-between p-6 text-center"
        >
          <span aria-hidden="true" className="kn-placeholder-pill h-1.5 w-14 rounded-full" />
          <div className="flex flex-col items-center gap-3">
            <span className="kn-accent-tonal inline-flex h-12 w-12 items-center justify-center rounded-xl">
              <Icon className="h-5 w-5" aria-hidden="true" />
            </span>
            <span className="text-sm font-bold kn-text-primary">{label}</span>
          </div>
          <span aria-hidden="true" className="kn-placeholder-pill h-1.5 w-24 rounded-full opacity-70" />
        </div>
      )}
    </div>
  );
}
