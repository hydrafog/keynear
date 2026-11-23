import { useEffect } from 'react';
import { useApp } from '../../state/AppProvider';
import { Laptop, X } from 'lucide-react';
import { INITIAL_SEND_QUEUE } from '../../data';
import { SectionHeading } from '../../design-system/Setting';
import { pluralize } from '../../lib/format';

const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus';

export function ShareSheet({ closing }: { closing: boolean }) {
  const { setShareSheetOpen, pairedDevices, sendQueue, formatBytes, startOutgoingTransfer } = useApp();
  const items = sendQueue.length ? sendQueue : INITIAL_SEND_QUEUE;
  const totalBytes = items.reduce((acc, i) => acc + i.sizeBytes, 0);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setShareSheetOpen(false);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [setShareSheetOpen]);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Android Share Sheet"
      onClick={(e) => {
        if (e.target === e.currentTarget) setShareSheetOpen(false);
      }}
      aria-hidden={closing || undefined}
      className={`${closing ? 'animate-kn-fade-out pointer-events-none' : 'animate-kn-fade-in'} absolute inset-0 z-40 flex flex-col justify-end kn-scrim`}
    >
      <div className={`${closing ? 'animate-kn-sheet-down' : 'animate-kn-sheet-up'} rounded-t-kn-3xl p-4 kn-sys-shade`}>
        <div className="mx-auto mb-3.5 h-1.5 w-12 rounded-full kn-sheet-handle" aria-hidden="true" />

        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-kn-lg font-kn-bold">Share {pluralize(items.length, 'item')}</h2>
            <p className="text-kn-xs tabular-nums kn-text-muted">{formatBytes(totalBytes)}</p>
          </div>
          <button
            type="button"
            onClick={() => setShareSheetOpen(false)}
            aria-label="Close Share Sheet"
            className={`flex h-12 w-12 cursor-pointer items-center justify-center rounded-full transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) kn-sys-notif-action ${FOCUS_RING}`}
          >
            <X className="h-5 w-5" aria-hidden="true" />
          </button>
        </div>

        <div className="mt-4">
          <SectionHeading>Direct Share via Keynear</SectionHeading>
          <div className="grid grid-cols-2 gap-3">
            {pairedDevices.map((dev) => (
              <button
                key={dev.deviceId}
                type="button"
                onClick={() => startOutgoingTransfer(dev, items)}
                className={`flex min-h-(--kn-size-116px) cursor-pointer flex-col items-center justify-center rounded-kn-3xl p-4 text-center transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) kn-sys-notif ${FOCUS_RING}`}
              >
                <div className="flex h-11 w-11 items-center justify-center rounded-full kn-sys-accent-tonal">
                  <Laptop className="h-5 w-5" aria-hidden="true" />
                </div>
                <span className="mt-2.5 w-full truncate text-kn-sm font-kn-semibold">{dev.displayName}</span>
                <span className="text-kn-xs kn-text-muted">{dev.isOnline ? 'Wi-Fi ready' : 'Bluetooth'}</span>
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
