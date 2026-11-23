import { useEffect } from 'react';
import { useApp } from '../../state/AppProvider';
import { NotificationHeader } from '../../design-system/Notifications';
import { VISIBILITY_LABELS } from '../../lib/format';
import { Lock, Unlock } from 'lucide-react';

const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus';

export function LockScreen({ closing }: { closing: boolean }) {
  const { setLockScreenOpen, setPendingUnlockAction, handleUnlockDevice } = useApp();

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setPendingUnlockAction(null);
        setLockScreenOpen(false);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [setPendingUnlockAction, setLockScreenOpen]);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Android Lock Screen"
      aria-hidden={closing || undefined}
      className={`${closing ? 'animate-kn-fade-out pointer-events-none' : 'animate-kn-fade-in'} absolute inset-0 z-50 flex flex-col justify-between p-4 kn-sys-shade`}
    >
      <div className="flex justify-end">
        <button
          type="button"
          onClick={() => {
            setPendingUnlockAction(null);
            setLockScreenOpen(false);
          }}
          className={`min-h-(--kn-size-48px) cursor-pointer rounded-full px-5 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) kn-sys-notif-action ${FOCUS_RING}`}
        >
          Dismiss
        </button>
      </div>

      <div className="text-center">
        <Lock className="mx-auto h-6 w-6 kn-text-muted" aria-hidden="true" />
        <div className="mt-3 text-kn-5xl font-kn-bold tracking-kn-tight tabular-nums">22:54</div>
        <div className="mt-1 text-kn-sm kn-text-muted">Thursday, November 12</div>
      </div>

      <section aria-label="Keynear unlock request" className="space-y-3 rounded-kn-3xl p-4 kn-sys-notif">
        <NotificationHeader label="Keynear · Locked" />
        <div className="text-kn-sm font-kn-bold">
          Unlock required for {VISIBILITY_LABELS.everyoneNearby.full}
        </div>
        <p className="text-kn-xs leading-kn-relaxed kn-text-secondary">
          While locked, the Quick Settings tile can switch between {VISIBILITY_LABELS.hidden.full} and{' '}
          {VISIBILITY_LABELS.pairedOnly.full}. Unlock your phone to allow discovery by unpaired devices.
        </p>
        <button
          type="button"
          onClick={handleUnlockDevice}
          className={`flex min-h-(--kn-size-54px) w-full cursor-pointer items-center justify-center gap-2.5 rounded-full px-5 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-98) kn-sys-accent ${FOCUS_RING}`}
        >
          <Unlock className="h-5 w-5" aria-hidden="true" />
          <span>Unlock with fingerprint</span>
        </button>
      </section>
    </div>
  );
}
