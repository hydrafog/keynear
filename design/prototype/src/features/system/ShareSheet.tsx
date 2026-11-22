import { useEffect } from 'react';
import { useApp } from '../../state/AppProvider';
import { Laptop, X } from 'lucide-react';
import { INITIAL_SEND_QUEUE } from '../../data';

export function ShareSheet() {
  const { setShareSheetOpen, pairedDevices, sendQueue, formatBytes, startOutgoingTransfer } = useApp();

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
      className="animate-kn-fade-in absolute inset-0 z-40 flex flex-col justify-end kn-scrim"
    >
      <div className="animate-kn-sheet-up rounded-t-3xl p-5 kn-fog-sheet">
        <div className="mx-auto mb-3.5 h-1.5 w-12 rounded-full kn-sheet-handle" aria-hidden="true" />

        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-kn-lg font-kn-bold">Share {sendQueue.length || 2} item(s)</h2>
            <p className="text-kn-xs tabular-nums kn-text-muted">
              {formatBytes(
                (sendQueue.length ? sendQueue : INITIAL_SEND_QUEUE).reduce(
                  (acc, i) => acc + i.sizeBytes,
                  0
                )
              )}
            </p>
          </div>
          <button
            type="button"
            onClick={() => setShareSheetOpen(false)}
            aria-label="Close Share Sheet"
            className="flex h-12 w-12 cursor-pointer items-center justify-center rounded-kn-xl transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500 kn-surface-tonal kn-surface-hover kn-text-secondary"
          >
            <X className="h-5 w-5" aria-hidden="true" />
          </button>
        </div>

        <div className="mt-4">
          <div className="mb-2.5 text-kn-xs font-kn-bold kn-text-muted">
            Direct Share via Keynear
          </div>
          <div className="grid grid-cols-2 gap-3">
            {pairedDevices.map((dev) => (
              <button
                key={dev.deviceId}
                type="button"
                onClick={() =>
                  startOutgoingTransfer(dev, sendQueue.length ? sendQueue : INITIAL_SEND_QUEUE)
                }
                className="flex min-h-(--kn-size-116px) flex-col items-center justify-center rounded-kn-2xl p-4 text-center transition-all duration-(--kn-duration-150) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500 kn-surface-tonal kn-surface-hover"
              >
                <div className="flex h-12 w-12 items-center justify-center rounded-kn-xl kn-loading-tonal">
                  <Laptop className="h-6 w-6" aria-hidden="true" />
                </div>
                <span className="mt-2.5 w-full truncate text-kn-sm font-kn-bold">{dev.displayName}</span>
                <span className="text-kn-xs kn-text-muted">
                  {dev.isOnline ? 'Wi-Fi ready' : 'Bluetooth'}
                </span>
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
