import { useApp } from '../../state/AppProvider';
import { Bluetooth, Wifi, Lock, Download, Upload, Signal, BatteryCharging } from 'lucide-react';

export function StatusBar() {
  const { shadeOpen, setShadeOpen, isDeviceLocked, config, activeTransfer, pendingShadeCount, activePct } = useApp();
  return (
        <button
          type="button"
          onClick={() => setShadeOpen(!shadeOpen)}
          aria-expanded={shadeOpen}
          aria-label="Open Android Quick Settings and Notification Shade"
          className="relative flex h-9 w-full shrink-0 cursor-pointer items-center justify-between px-4 text-kn-xs font-kn-semibold select-none transition-opacity duration-(--kn-duration-150) before:absolute before:inset-x-0 before:top-0 before:h-11 before:content-[''] active:opacity-75 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-kn-focus"
        >
          <div className="flex items-center gap-2">
            <span className="tabular-nums">22:54</span>

            {activeTransfer && (
              <span className="animate-kn-fade-in inline-flex items-center gap-1 rounded-full kn-sys-accent px-2 py-0.5 text-kn-xs font-kn-bold tabular-nums">
                {activeTransfer.direction === 'receive' ? (
                  <Download className="h-3 w-3" aria-hidden="true" />
                ) : (
                  <Upload className="h-3 w-3" aria-hidden="true" />
                )}
                <span>{activePct}%</span>
              </span>
            )}
            {!activeTransfer && pendingShadeCount > 0 && (
              <span className="inline-flex h-4 min-w-4 items-center justify-center rounded-full kn-sys-accent px-1 text-kn-xs font-kn-bold tabular-nums">
                {pendingShadeCount}
              </span>
            )}
          </div>

          <div className="flex items-center gap-1.5">
            {isDeviceLocked && <Lock className="h-3.5 w-3.5" aria-hidden="true" />}
            {config.visibilityMode !== 'hidden' && (
              <Bluetooth className="h-3.5 w-3.5" aria-hidden="true" />
            )}
            <Wifi className="h-3.5 w-3.5" aria-hidden="true" />
            <Signal className="h-3.5 w-3.5" aria-hidden="true" />
            <BatteryCharging className="h-4 w-4" aria-hidden="true" />
          </div>
        </button>
  );
}
