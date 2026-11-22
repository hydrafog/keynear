import { useEffect } from 'react';
import { useApp } from '../../state/AppProvider';
import {
  Wifi, Shield, ShieldCheck, X, Key, Clipboard, ChevronDown,
  ChevronUp, Download, Upload, Radio, EyeOff, Sun, Moon,
} from 'lucide-react';
import { QuickSettingsTile, ShadeTriggerButton } from '../../design-system/Controls';
import { NotificationCard, NotificationHeader } from '../../design-system/Notifications';
import { CallAlertCard, MediaCarouselCard, MirroredNotificationCard } from './CarinaCards';

const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500';
const SYS_PILL = `min-h-(--kn-size-44px) cursor-pointer rounded-full px-4 text-kn-xs font-kn-semibold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) ${FOCUS_RING}`;
const SYS_ICON_BTN = `flex h-8 w-8 cursor-pointer items-center justify-center rounded-full kn-sys-notif-action ${FOCUS_RING}`;
const NOTIF_RING_STYLE = { boxShadow: '0 0 0 2px var(--kn-sys-notif-bg)' };

export function Shade() {
  const {
    darkMode, setDarkMode, setActiveTab, setSubScreen, setShadeOpen, setLockScreenOpen,
    setIsDeviceLocked, config, pairedDevices, everyoneNearbySecondsLeft, transfers,
    expandedNotifIds, setExpandedNotifIds, pairingPrompt, otpPrompt, setOtpPrompt, formatBytes,
    formatCountdown, handleCycleQuickSettingsTile, triggerIncomingDrop, handleTransferConsent,
    handleCancelTransfer, dismissTransferNotification, triggerBlePairing, handlePairingDecision,
    triggerSmsOtpConsent, handlePushOtp, handleSendClipboardFromTile,
    triggerMirroredNotification, triggerIncomingCall,
  } = useApp();

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setShadeOpen(false);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [setShadeOpen]);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Android Quick Settings and Notification Shade"
      className="animate-kn-shade-down absolute inset-0 z-40 flex flex-col transition-colors duration-(--kn-duration-200) kn-sys-shade"
    >
      <div className="flex h-14 shrink-0 items-center justify-between px-4 text-kn-xs font-kn-semibold">
        <div className="flex items-center gap-2">
          <span className="tabular-nums text-kn-sm font-kn-bold">22:54</span>
          <span className="kn-text-muted">Thu, Nov 12</span>
        </div>
        <button type="button" onClick={() => setShadeOpen(false)} className={`flex items-center gap-1.5 kn-sys-notif-action ${SYS_PILL}`}>
          <span>Close shade</span>
          <ChevronUp className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>

      <div className="flex-1 space-y-3 overflow-y-auto px-3.5 pb-6 pt-1">
        <div className="grid grid-cols-2 gap-2.5">
          <div className="flex min-h-(--kn-size-68px) items-center gap-3 rounded-full px-4 py-3 kn-sys-qs-on">
            <Wifi className="h-5 w-5 shrink-0" aria-hidden="true" />
            <div className="min-w-0">
              <div className="truncate text-kn-sm font-kn-bold">Internet</div>
              <div className="truncate text-kn-xs opacity-90">Home-5G</div>
            </div>
          </div>

          <QuickSettingsTile className="kn-sys-qs-off kn-sys-theme-tile" ariaPressed={darkMode} onClick={() => setDarkMode(!darkMode)}>
            <Moon className="kn-when-dark h-5 w-5 shrink-0" aria-hidden="true" />
            <Sun className="kn-when-light h-5 w-5 shrink-0 text-kn-amber-500" aria-hidden="true" />
            <div className="min-w-0">
              <div className="truncate text-kn-sm font-kn-bold">Dark theme</div>
              <div className="truncate text-kn-xs opacity-85">
                <span className="kn-when-dark">On</span>
                <span className="kn-when-light">Off</span>
              </div>
            </div>
          </QuickSettingsTile>

          <QuickSettingsTile
            className={config.visibilityMode !== 'hidden' ? 'kn-sys-qs-on' : 'kn-sys-qs-off'}
            onClick={handleCycleQuickSettingsTile}
          >
            {config.visibilityMode === 'hidden' ? (
              <EyeOff className="h-5 w-5 shrink-0" aria-hidden="true" />
            ) : config.visibilityMode === 'everyoneNearby' ? (
              <Radio className="h-5 w-5 shrink-0" aria-hidden="true" />
            ) : (
              <ShieldCheck className="h-5 w-5 shrink-0" aria-hidden="true" />
            )}
            <div className="min-w-0">
              <div className="truncate text-kn-sm font-kn-bold">Keynear</div>
              <div className="truncate text-kn-xs tabular-nums opacity-85">
                {config.visibilityMode === 'hidden' && 'Off'}
                {config.visibilityMode === 'pairedOnly' && 'Paired only'}
                {config.visibilityMode === 'everyoneNearby' && `Everyone (${formatCountdown(everyoneNearbySecondsLeft)})`}
              </div>
            </div>
          </QuickSettingsTile>

          <QuickSettingsTile className="kn-sys-qs-off" onClick={handleSendClipboardFromTile}>
            <Clipboard className="h-5 w-5 shrink-0 kn-loading-text" aria-hidden="true" />
            <div className="min-w-0">
              <div className="truncate text-kn-sm font-kn-bold">Send clipboard</div>
              <div className="truncate text-kn-xs kn-text-muted">To desktop</div>
            </div>
          </QuickSettingsTile>
        </div>

        <MediaCarouselCard />

        <div className="space-y-2.5 pt-0.5">
          <CallAlertCard />
          <MirroredNotificationCard />

          {transfers.map((tx) => {
            const pct = Math.min(100, Math.round((tx.transferredBytes / tx.totalBytes) * 100));
            const isExpanded = expandedNotifIds[tx.transferId] ?? true;
            const countLabel = `${tx.items.length} ${tx.items.length === 1 ? 'item' : 'items'}`;
            const title = tx.direction === 'receive'
              ? tx.state === 'completed' ? `Received ${countLabel} from ${tx.peerName}` : `${tx.peerName} wants to send ${countLabel}`
              : tx.state === 'completed' ? `Sent ${countLabel} to ${tx.peerName}` : `Sending ${countLabel} to ${tx.peerName}`;

            return (
              <NotificationCard key={tx.transferId}>
                <div className="flex items-center justify-between gap-2">
                  <NotificationHeader
                    icon={tx.direction === 'receive' ? <Download className="h-3 w-3" aria-hidden="true" /> : <Upload className="h-3 w-3" aria-hidden="true" />}
                    label={tx.state === 'active' ? 'Keynear · Live Update' : 'Keynear · now'}
                  />
                  <div className="flex items-center gap-1.5">
                    <button type="button" aria-label="Expand or collapse notification details" onClick={() => setExpandedNotifIds((prev) => ({ ...prev, [tx.transferId]: !isExpanded }))} className={SYS_ICON_BTN}>
                      {isExpanded ? <ChevronUp className="h-4 w-4" aria-hidden="true" /> : <ChevronDown className="h-4 w-4" aria-hidden="true" />}
                    </button>
                    {tx.state !== 'active' && (
                      <button type="button" aria-label="Dismiss notification" onClick={() => dismissTransferNotification(tx.transferId)} className={SYS_ICON_BTN}>
                        <X className="h-4 w-4 kn-error-text" aria-hidden="true" />
                      </button>
                    )}
                  </div>
                </div>

                <div className="mt-2 flex items-baseline justify-between gap-2">
                  <div className="text-kn-sm font-kn-bold">{title}</div>
                  <span className="shrink-0 text-kn-xs font-kn-bold tabular-nums kn-loading-text">
                    {tx.state === 'active' ? `${pct}%` : formatBytes(tx.totalBytes)}
                  </span>
                </div>

                {tx.state === 'active' && (
                  <div className="mt-3">
                    <div role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100} aria-label="Transfer progress" className="relative flex items-center">
                      <div className="grid w-full grid-cols-12 gap-1">
                        <div className="col-span-2 h-1.5 overflow-hidden rounded-full kn-sys-notif-action">
                          <div className="h-full kn-sys-qs-on transition-[width] duration-(--kn-duration-500) ease-out" style={{ width: `${Math.min(100, (pct / 15) * 100)}%` }} />
                        </div>
                        <div className="col-span-8 h-1.5 overflow-hidden rounded-full kn-sys-notif-action">
                          <div className="h-full kn-sys-qs-on transition-[width] duration-(--kn-duration-500) ease-out" style={{ width: `${pct <= 15 ? 0 : Math.min(100, ((pct - 15) / 75) * 100)}%` }} />
                        </div>
                        <div className="col-span-2 h-1.5 overflow-hidden rounded-full kn-sys-notif-action">
                          <div className="h-full bg-kn-emerald-500 transition-[width] duration-(--kn-duration-500) ease-out" style={{ width: `${pct <= 90 ? 0 : Math.min(100, ((pct - 90) / 10) * 100)}%` }} />
                        </div>
                      </div>
                      <span aria-hidden="true" style={NOTIF_RING_STYLE} className={`pointer-events-none absolute left-[16.6%] h-2 w-2 -translate-x-1/2 rounded-full ${pct >= 15 ? 'kn-sys-qs-on' : 'kn-sys-notif-action'}`} />
                      <span aria-hidden="true" style={NOTIF_RING_STYLE} className={`pointer-events-none absolute left-[83.3%] h-2 w-2 -translate-x-1/2 rounded-full ${pct >= 90 ? 'bg-kn-emerald-500' : 'kn-sys-notif-action'}`} />
                      <span aria-hidden="true" style={{ left: `${pct}%`, ...NOTIF_RING_STYLE }} className="pointer-events-none absolute h-3.5 w-3.5 -translate-x-1/2 rounded-full kn-sys-qs-on transition-[left] duration-(--kn-duration-500) ease-out" />
                    </div>
                    <div className="mt-2 text-kn-xs tabular-nums kn-text-muted">
                      42.4 MB/s · {formatBytes(tx.transferredBytes)} of {formatBytes(tx.totalBytes)}
                    </div>
                  </div>
                )}

                {isExpanded && (
                  <div className="animate-kn-fade-in mt-2 space-y-1 text-kn-xs kn-text-secondary">
                    {tx.items.map((item, idx) => (
                      <div key={idx} className="flex items-center justify-between gap-2">
                        <span className="truncate">{item.name}</span>
                        <span className="shrink-0 tabular-nums kn-text-muted">{formatBytes(item.sizeBytes)}</span>
                      </div>
                    ))}
                  </div>
                )}

                {tx.state === 'pendingConsent' && (
                  <div className="mt-3 flex items-center gap-2">
                    <button type="button" onClick={() => handleTransferConsent(tx.transferId, true)} className={`font-kn-bold kn-sys-qs-on ${SYS_PILL}`}>Accept</button>
                    <button type="button" onClick={() => handleTransferConsent(tx.transferId, false)} className={`kn-sys-notif-action ${SYS_PILL}`}>Decline</button>
                  </div>
                )}

                {tx.state === 'active' && (
                  <div className="mt-3 flex">
                    <button type="button" onClick={() => handleCancelTransfer(tx.transferId)} className={`kn-sys-notif-action kn-error-text ${SYS_PILL}`}>Cancel transfer</button>
                  </div>
                )}
              </NotificationCard>
            );
          })}

          {pairingPrompt && (
            <NotificationCard>
              <div className="flex items-center justify-between gap-2">
                <NotificationHeader icon={<Shield className="h-3 w-3" aria-hidden="true" />} label="Keynear · Pairing" />
                <span className="text-kn-xs font-kn-bold tabular-nums text-kn-amber-500">{pairingPrompt.secondsLeft}s</span>
              </div>
              <div className="mt-2 text-kn-sm font-kn-bold">Pair with {pairingPrompt.peerName}?</div>
              <p className="mt-0.5 text-kn-xs kn-text-muted">Confirm this 6-digit code matches the notification on your Linux desktop.</p>
              <div className="mt-2 text-kn-2xl font-kn-bold tracking-kn-code tabular-nums">{pairingPrompt.verificationCode}</div>
              <div className="mt-3 flex items-center gap-2">
                <button type="button" onClick={() => handlePairingDecision(true)} className={`font-kn-bold kn-sys-qs-on ${SYS_PILL}`}>Approve</button>
                <button type="button" onClick={() => handlePairingDecision(false)} className={`kn-sys-notif-action ${SYS_PILL}`}>Reject</button>
              </div>
            </NotificationCard>
          )}

          {otpPrompt && (
            <NotificationCard>
              <div className="flex items-center justify-between gap-2">
                <NotificationHeader icon={<Key className="h-3 w-3" aria-hidden="true" />} label="Keynear · 2FA" />
                <button type="button" aria-label="Dismiss 2FA notification" onClick={() => setOtpPrompt(null)} className={SYS_ICON_BTN}>
                  <X className="h-4 w-4 kn-error-text" aria-hidden="true" />
                </button>
              </div>
              <div className="mt-2 flex items-baseline justify-between gap-2">
                <span className="text-kn-xl font-kn-bold tracking-kn-wider tabular-nums">{otpPrompt.code}</span>
                <span className="text-kn-xs kn-text-muted">From {otpPrompt.sender}</span>
              </div>
              <div className="mt-3 flex">
                <button type="button" onClick={handlePushOtp} className={`kn-sys-notif-action kn-loading-text ${SYS_PILL}`}>Copy to {otpPrompt.targetDesktop}</button>
              </div>
            </NotificationCard>
          )}

          <article className="flex items-center justify-between gap-3 rounded-3xl px-4 py-3 kn-sys-notif">
            <div className="flex min-w-0 items-center gap-3">
              <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full kn-sys-qs-on">
                <Radio className="h-3 w-3" aria-hidden="true" />
              </span>
              <div className="min-w-0">
                <div className="truncate text-kn-sm font-kn-bold">Keynear background service</div>
                <div className="truncate text-kn-xs kn-text-muted">
                  {config.visibilityMode === 'hidden'
                    ? 'Hidden, tap Keynear tile to enable'
                    : `${pairedDevices.length} paired ${pairedDevices.length === 1 ? 'computer' : 'computers'} ready`}
                </div>
              </div>
            </div>
            <button type="button" onClick={() => { setShadeOpen(false); setSubScreen(null); setActiveTab('settings'); }} className={`shrink-0 kn-sys-notif-action ${SYS_PILL}`}>
              Settings
            </button>
          </article>

          <div className="pt-2">
            <div className="mb-2 px-1 text-kn-xs font-kn-semibold kn-text-muted">Prototype simulator</div>
            <div className="grid grid-cols-2 gap-2">
              <ShadeTriggerButton onClick={() => triggerIncomingDrop(false)}>+ Incoming drop</ShadeTriggerButton>
              <ShadeTriggerButton onClick={() => triggerBlePairing('fedora-studio')}>+ 6-digit pairing</ShadeTriggerButton>
              <ShadeTriggerButton onClick={triggerSmsOtpConsent}>+ SMS 2FA</ShadeTriggerButton>
              <ShadeTriggerButton onClick={triggerMirroredNotification}>+ Mirrored notification</ShadeTriggerButton>
              <ShadeTriggerButton onClick={triggerIncomingCall}>+ Incoming call</ShadeTriggerButton>
              <ShadeTriggerButton onClick={() => { setShadeOpen(false); setIsDeviceLocked(true); setLockScreenOpen(true); }}>Lock screen</ShadeTriggerButton>
            </div>
          </div>
        </div>
      </div>

      <div className="flex justify-center pb-3 pt-1">
        <button type="button" onClick={() => setShadeOpen(false)} aria-label="Close notification shade" className={`h-1.5 w-32 cursor-pointer rounded-full ${FOCUS_RING} kn-gesture-pill`} />
      </div>
    </div>
  );
}
