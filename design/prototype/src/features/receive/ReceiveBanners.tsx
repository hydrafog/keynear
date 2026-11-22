import { useApp } from '../../state/AppProvider';
import type { TransferSession } from '../../types';

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
  } = useApp();

  const activeTransfers = transfers
    .filter((t) => t.state === 'pendingConsent' || t.state === 'active')
    .slice(0, 2);

  if (!pairingPrompt && activeTransfers.length === 0) return null;

  return (
    <div className="space-y-2.5">
      {pairingPrompt && (
        <section
          aria-label={`Pair with ${pairingPrompt.peerName}`}
          className="animate-kn-modal-in space-y-2.5 rounded-kn-2xl p-4 kn-fog-surface"
        >
          <div className="text-center text-kn-xs font-kn-bold tracking-kn-wider kn-text-muted">
            PAIR WITH {pairingPrompt.peerName.toUpperCase()}
          </div>
          <div className="rounded-kn-xl py-2.5 text-center text-kn-2xl font-kn-bold tracking-kn-code tabular-nums kn-track-well">
            {pairingPrompt.verificationCode}
          </div>
          <p className="text-center text-kn-xs tabular-nums kn-text-muted">
            Confirm this code matches ({pairingPrompt.secondsLeft}s)
          </p>
          <div className="flex gap-2.5">
            <button
              type="button"
              onClick={() => handlePairingDecision(false)}
              className="min-h-(--kn-size-44px) flex-1 rounded-kn-xl px-4 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500 kn-surface-tonal"
            >
              Decline
            </button>
            <button
              type="button"
              onClick={() => handlePairingDecision(true)}
              className="min-h-(--kn-size-44px) flex-1 rounded-kn-xl px-4 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500 kn-loading-bg"
            >
              Pair
            </button>
          </div>
        </section>
      )}

      {activeTransfers.length > 0 && (
        <section
          aria-label="Active transfers"
          className="animate-kn-modal-in space-y-4 rounded-kn-2xl p-5 kn-fog-surface"
        >
          {activeTransfers.map((tx) => {
            const pct =
              tx.totalBytes > 0
                ? Math.min(100, Math.round((tx.transferredBytes / tx.totalBytes) * 100))
                : 0;
            const directionLabel = tx.direction === 'send' ? 'To' : 'From';

            return (
              <div key={tx.transferId} className="space-y-3.5">
                <div className="flex items-baseline justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-kn-base font-kn-bold">
                      {summarizeTransfer(tx)}
                    </div>
                    <div className="mt-0.5 truncate text-kn-xs tabular-nums kn-text-muted">
                      {directionLabel} {tx.peerName} · {formatBytes(tx.totalBytes)}
                    </div>
                  </div>
                  {tx.state === 'active' && (
                    <span className="shrink-0 text-kn-xs font-kn-bold tabular-nums kn-loading-text">
                      {pct}%
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
                    <button
                      type="button"
                      onClick={() => handleTransferConsent(tx.transferId, false)}
                      className="min-h-(--kn-size-48px) flex-1 cursor-pointer rounded-kn-xl px-4 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500 kn-surface-tonal kn-surface-hover kn-text-secondary"
                    >
                      Decline
                    </button>
                    <button
                      type="button"
                      onClick={() => handleTransferConsent(tx.transferId, true)}
                      className="min-h-(--kn-size-48px) flex-1 cursor-pointer rounded-kn-xl px-4 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500 kn-loading-bg"
                    >
                      Accept
                    </button>
                  </div>
                ) : (
                  <button
                    type="button"
                    onClick={() => handleCancelTransfer(tx.transferId)}
                    className="min-h-(--kn-size-48px) w-full cursor-pointer rounded-kn-xl px-4 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500 kn-surface-tonal kn-surface-hover kn-text-secondary"
                  >
                    Cancel transfer
                  </button>
                )}
              </div>
            );
          })}
        </section>
      )}
    </div>
  );
}

