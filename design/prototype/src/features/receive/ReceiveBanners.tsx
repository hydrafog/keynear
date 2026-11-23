import { useApp } from '../../state/AppProvider';
import { ActionButton } from '../../design-system/Controls';
import { Card } from '../../design-system/Card';
import { SectionHeading } from '../../design-system/Setting';
import { joinMeta } from '../../lib/format';
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

      {activeTransfers.length > 0 && (
        <Card label="Active transfers" className="animate-kn-modal-in space-y-4">
          {activeTransfers.map((tx) => {
            const pct =
              tx.totalBytes > 0
                ? Math.min(100, Math.round((tx.transferredBytes / tx.totalBytes) * 100))
                : 0;
            const directionLabel = tx.direction === 'send' ? 'To' : 'From';

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
                ) : (
                  <ActionButton tone="tonal" height={46} width="block" onClick={() => handleCancelTransfer(tx.transferId)}>
                    Cancel transfer
                  </ActionButton>
                )}
              </div>
            );
          })}
        </Card>
      )}
    </div>
  );
}
