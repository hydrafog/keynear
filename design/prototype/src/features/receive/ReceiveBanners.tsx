import { useEffect, useRef, useState, type PointerEvent } from 'react';
import { useApp } from '../../state/AppProvider';
import { ActionButton } from '../../design-system/Controls';
import { Card } from '../../design-system/Card';
import { SectionHeading } from '../../design-system/Setting';
import { joinMeta } from '../../lib/format';
import type { TransferSession } from '../../types';

const DISMISS_THRESHOLD_PX = 88;
const AUTO_DISMISS_MS = 3500;
const EXIT_MS = 220;
const EXIT_EASING = 'cubic-bezier(0.4, 0, 1, 1)';
const DRAG_PX = 6;

function summarizeTransfer(tx: TransferSession): string {
  const first = tx.items[0]?.name ?? 'Files';
  if (tx.items.length <= 1) return first;
  return `${first} +${tx.items.length - 1}`;
}

export function ReceiveBanners() {
  const {
    pairingPrompt,
    handlePairingDecision,
    transfers,
    handleTransferConsent,
    handleCancelTransfer,
    formatBytes,
    dismissedTransferIds,
    dismissTransfers,
  } = useApp();

  const [dragX, setDragX] = useState(0);
  const [exiting, setExiting] = useState(false);
  const swipe = useRef({ active: false, id: -1, x: 0 });
  const exitTimer = useRef(0);

  const visibleTransfers = transfers
    .filter((t) => !dismissedTransferIds.has(t.transferId))
    .slice(0, 2);
  const lastTransfers = visibleTransfers.length ? visibleTransfers : [];

  const terminalKey =
    visibleTransfers.length > 0 &&
    visibleTransfers.every((t) => t.state === 'completed' || t.state === 'cancelled')
      ? visibleTransfers.map((t) => t.transferId).join(',')
      : '';

  useEffect(() => {
    if (!terminalKey) return;
    const timer = window.setTimeout(() => {
      setExiting(true);
      exitTimer.current = window.setTimeout(() => {
        dismissTransfers(terminalKey.split(','));
        setExiting(false);
        setDragX(0);
      }, EXIT_MS);
    }, AUTO_DISMISS_MS);
    return () => {
      window.clearTimeout(timer);
      window.clearTimeout(exitTimer.current);
    };
  }, [terminalKey, dismissTransfers]);

  const dismiss = (ids: string[]) => {
    setExiting(true);
    exitTimer.current = window.setTimeout(() => {
      dismissTransfers(ids);
      setExiting(false);
      setDragX(0);
    }, EXIT_MS);
  };

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    swipe.current = { active: true, id: e.pointerId, x: e.clientX };
  };
  const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
    const st = swipe.current;
    if (!st.active || st.id !== e.pointerId) return;
    const dx = e.clientX - st.x;
    if (dx <= 0 && dragX === 0) return;
    if (Math.abs(dx) < DRAG_PX && dragX === 0) return;
    if (!e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.setPointerCapture(e.pointerId);
    setDragX(Math.max(0, dx));
  };
  const onPointerUp = (e: PointerEvent<HTMLDivElement>) => {
    const st = swipe.current;
    if (!st.active || st.id !== e.pointerId) return;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
    st.active = false;
    if (dragX >= DISMISS_THRESHOLD_PX) dismiss(lastTransfers.map((tx) => tx.transferId));
    else setDragX(0);
  };

  if (!pairingPrompt && lastTransfers.length === 0) return null;

  return (
    <div className="space-y-2.5">
      {pairingPrompt && (
        <Card label={`Pair with ${pairingPrompt.peerName}`} className="animate-kn-modal-in space-y-3">
          <SectionHeading className="text-center">Pair with {pairingPrompt.peerName}</SectionHeading>
          <div className="rounded-kn-xl py-2.5 text-center text-kn-2xl font-kn-bold font-kn-mono tracking-kn-code tabular-nums kn-track-well">
            {pairingPrompt.verificationCode}
          </div>
          <p className="text-center text-kn-xs tabular-nums kn-text-muted">
            Confirm this code matches ({pairingPrompt.secondsLeft}s)
          </p>
          <div className="flex gap-2.5">
            <ActionButton tone="tonal" height={46} width="grow" onClick={() => handlePairingDecision(false)}>
              Decline
            </ActionButton>
            <ActionButton tone="accent" height={46} width="grow" onClick={() => handlePairingDecision(true)}>
              Pair
            </ActionButton>
          </div>
        </Card>
      )}

      {lastTransfers.length > 0 && (
        <div
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
          style={{
            transform: `translateX(${exiting ? 520 : dragX}px)`,
            opacity: exiting ? 0 : Math.max(0, 1 - dragX / 360),
            transition: exiting
              ? `transform ${EXIT_MS}ms ${EXIT_EASING}, opacity ${EXIT_MS}ms ${EXIT_EASING}`
              : dragX === 0
              ? 'transform 150ms cubic-bezier(0.22, 1, 0.36, 1), opacity 150ms cubic-bezier(0.22, 1, 0.36, 1)'
              : 'none',
          }}
          className="animate-kn-modal-in touch-pan-y"
        >
          <Card label="Active transfers" className="space-y-4">
            {lastTransfers.map((tx) => {
              const pct =
                tx.totalBytes > 0
                  ? Math.min(100, Math.round((tx.transferredBytes / tx.totalBytes) * 100))
                  : 0;
              const directionLabel = tx.direction === 'send' ? 'To' : 'From';
              const terminal = tx.state === 'completed' || tx.state === 'cancelled';

              return (
                <div key={tx.transferId} className="space-y-3">
                  <div className="flex items-baseline justify-between gap-3">
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-kn-base font-kn-semibold">
                        {summarizeTransfer(tx)}
                      </div>
                      <div className="mt-0.5 truncate text-kn-xs tabular-nums kn-text-muted">
                        {joinMeta(`${directionLabel} ${tx.peerName}`, formatBytes(tx.totalBytes))}
                      </div>
                    </div>
                    {tx.state === 'active' && (
                      <span className="shrink-0 text-kn-xs font-kn-bold tabular-nums kn-loading-text">
                        {pct}%
                      </span>
                    )}
                    {terminal && (
                      <span className="shrink-0 text-kn-xs font-kn-bold kn-loading-text">
                        {tx.state === 'completed' ? 'Completed' : 'Cancelled'}
                      </span>
                    )}
                  </div>

                  {tx.state === 'active' && (
                    <div
                      role="progressbar"
                      aria-valuenow={pct}
                      aria-valuemin={0}
                      aria-valuemax={100}
                      aria-label={`Transfer progress for ${summarizeTransfer(tx)}`}
                      className="h-2.5 w-full overflow-hidden rounded-full kn-track-well"
                    >
                      <div
                        className="h-full rounded-full transition-[width] duration-(--kn-duration-morph) ease-(--ease-kn-spring) kn-loading-bg"
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                  )}

                  {tx.state === 'pendingConsent' ? (
                    <div className="flex gap-2.5">
                      <ActionButton tone="tonal" height={46} width="grow" onClick={() => handleTransferConsent(tx.transferId, false)}>
                        Decline
                      </ActionButton>
                      <ActionButton tone="accent" height={46} width="grow" onClick={() => handleTransferConsent(tx.transferId, true)}>
                        Accept
                      </ActionButton>
                    </div>
                  ) : tx.state === 'active' ? (
                    <ActionButton tone="tonal" height={46} width="block" onClick={() => handleCancelTransfer(tx.transferId)}>
                      Cancel transfer
                    </ActionButton>
                  ) : null}
                </div>
              );
            })}
          </Card>
        </div>
      )}
    </div>
  );
}
