import { useEffect, useState } from 'react';

export interface WireframeSlotProps {
  src: string;
  alt: string;
}

export function WireframeSlot({ src, alt }: WireframeSlotProps) {
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setFailed(false);
  }, [src]);

  const showImage = Boolean(src) && !failed;

  return (
    <div className="mb-8 flex h-44 w-full items-center justify-center sm:h-52">
      {showImage ? (
        <img
          src={src}
          alt={alt}
          onError={() => setFailed(true)}
          className="kn-wf-art max-h-full max-w-full object-contain"
        />
      ) : (
        <div
          role="img"
          aria-label={alt}
          className="kn-wf-slot flex h-36 w-full max-w-[260px] items-center justify-center rounded-2xl"
        >
          <span className="font-mono text-xs kn-text-muted">Wireframe slot</span>
        </div>
      )}
    </div>
  );
}
