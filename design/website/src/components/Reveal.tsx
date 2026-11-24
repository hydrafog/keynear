import type { CSSProperties, ReactNode } from 'react';
import { useEffect, useState } from 'react';

type KnDelayStyle = CSSProperties & {
  '--kn-delay': string;
};

function useReveal(threshold = 0.12): [boolean, (node: HTMLElement | null) => void] {
  const [element, setElement] = useState<HTMLElement | null>(null);
  const [visible, setVisible] = useState(() => typeof IntersectionObserver === 'undefined');

  useEffect(() => {
    if (visible || !element) {
      return;
    }

    if (typeof IntersectionObserver === 'undefined') {
      setVisible(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { threshold, rootMargin: '0px 0px -40px 0px' },
    );

    observer.observe(element);
    return () => observer.disconnect();
  }, [element, threshold, visible]);

  return [visible, setElement];
}

export interface RevealProps {
  children: ReactNode;
  delay?: number;
  className?: string;
  as?: 'div' | 'li' | 'figure';
}

export function Reveal({ children, delay = 0, className = '', as: Tag = 'div' }: RevealProps) {
  const [visible, setElement] = useReveal();
  const style: KnDelayStyle = { '--kn-delay': `${delay}ms` };

  return (
    <Tag
      ref={setElement}
      style={style}
      className={`kn-reveal ${visible ? 'is-visible' : ''} ${className}`}
    >
      {children}
    </Tag>
  );
}
