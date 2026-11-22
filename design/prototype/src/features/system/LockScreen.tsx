import { useEffect } from 'react';
import { useApp } from '../../state/AppProvider';
import { KeynearMark } from '../../design-system/KeynearMark';
import { Shield, Lock, Unlock } from 'lucide-react';

export function LockScreen() {
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
      className="animate-kn-fade-in absolute inset-0 z-50 flex flex-col justify-between p-6 kn-canvas"
    >
      <div className="flex justify-end">
        <button
          type="button"
          onClick={() => {
            setPendingUnlockAction(null);
            setLockScreenOpen(false);
          }}
          className="min-h-(--kn-size-48px) rounded-kn-xl px-5 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500 kn-surface"
        >
          Dismiss
        </button>
      </div>

      <div className="text-center">
        <KeynearMark size={36} variant="brand" className="mx-auto mb-3" />
        <Lock className="mx-auto h-6 w-6 kn-text-muted" aria-hidden="true" />
        <div className="mt-3 text-kn-5xl font-kn-bold tracking-kn-tight tabular-nums">22:54</div>
        <div className="mt-1 text-kn-sm kn-text-muted">Thursday, November 12</div>
      </div>

      <div className="space-y-3.5 rounded-kn-2xl p-5 kn-surface">
        <div className="flex items-center gap-2 text-kn-sm font-kn-bold text-kn-amber-500">
          <Shield className="h-5 w-5 shrink-0" aria-hidden="true" />
          <span>Unlock required for Everyone nearby</span>
        </div>
        <p className="text-kn-xs leading-kn-relaxed kn-text-secondary">
          While locked, the Quick Settings tile can switch between Off and Paired only. Unlock your
          phone to allow discovery by unpaired devices.
        </p>
        <button
          type="button"
          onClick={handleUnlockDevice}
          className="flex min-h-(--kn-size-54px) w-full items-center justify-center gap-2.5 rounded-kn-xl px-5 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-98) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500 kn-loading-bg"
        >
          <Unlock className="h-5 w-5" aria-hidden="true" />
          <span>Unlock with fingerprint</span>
        </button>
      </div>
    </div>
  );
}
