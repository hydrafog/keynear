import { useEffect, type FormEvent } from 'react';
import { useApp } from '../../state/AppProvider';
import { ActionButton, SelectionTile } from '../../design-system/Controls';
import { Card, ListGroup } from '../../design-system/Card';
import { SectionHeading } from '../../design-system/Setting';
import { joinMeta, pluralize } from '../../lib/format';
import { Laptop, X, Folder, FileText, Clipboard, Send, Plus, Share2 } from 'lucide-react';

const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus';

export function SendScreen() {
  const {
    pairedDevices,
    nearbyPeers,
    sendQueue,
    setSendQueue,
    showTextComposer,
    setShowTextComposer,
    setShareSheetOpen,
    customTextSnippet,
    setCustomTextSnippet,
    formatBytes,
    startOutgoingTransfer,
    transfers,
    handleTransferConsent,
    handleCancelTransfer,
  } = useApp();

  useEffect(() => {
    if (!showTextComposer) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setShowTextComposer(false);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [showTextComposer, setShowTextComposer]);

  const handleAddText = (e: FormEvent) => {
    e.preventDefault();
    const trimmed = customTextSnippet.trim();
    if (!trimmed) return;
    setSendQueue((prev) => [
      ...prev,
      {
        id: `txt-${Date.now()}`,
        name: trimmed.slice(0, 28),
        sizeBytes: trimmed.length,
        isDirectory: false,
      },
    ]);
    setCustomTextSnippet('');
    setShowTextComposer(false);
  };

  const totalBytes = sendQueue.reduce((acc, i) => acc + i.sizeBytes, 0);
  const outgoingTransfers = transfers
    .filter((t) => t.direction === 'send' && (t.state === 'active' || t.state === 'pendingConsent'))
    .slice(0, 2);

  return (
    <div className="space-y-6">
      <section>
        <SectionHeading>Selection</SectionHeading>

        <div className="grid grid-cols-2 gap-3.5">
          <SelectionTile
            icon={<FileText className="h-8 w-8 kn-loading-text" aria-hidden="true" />}
            label="File"
            onClick={() =>
              setSendQueue((prev) => [
                ...prev,
                {
                  id: `file-${Date.now()}`,
                  name: `document-${prev.length + 1}.pdf`,
                  sizeBytes: 3_450_000,
                  isDirectory: false,
                },
              ])
            }
          />

          <SelectionTile
            icon={<Folder className="h-8 w-8 kn-loading-text" aria-hidden="true" />}
            label="Folder"
            onClick={() =>
              setSendQueue((prev) => [
                ...prev,
                {
                  id: `dir-${Date.now()}`,
                  name: `photos-${prev.length + 1}/`,
                  sizeBytes: 64_200_000,
                  isDirectory: true,
                },
              ])
            }
          />

          <SelectionTile
            icon={<Clipboard className="h-8 w-8 kn-loading-text" aria-hidden="true" />}
            label="Clipboard"
            onClick={() =>
              setSendQueue((prev) => [
                ...prev,
                {
                  id: `clip-${Date.now()}`,
                  name: 'clipboard-text.txt',
                  sizeBytes: 1_240,
                  isDirectory: false,
                },
              ])
            }
          />

          <SelectionTile
            icon={<Plus className="h-8 w-8 kn-loading-text" aria-hidden="true" />}
            label="Text"
            onClick={() => setShowTextComposer(true)}
          />
        </div>
      </section>

      <Card>
        {sendQueue.length === 0 ? (
          <div className="flex flex-col items-center gap-2 px-4 py-5 text-center">
            <span className="flex h-11 w-11 items-center justify-center rounded-kn-xl kn-loading-tonal">
              <FileText className="h-5 w-5" aria-hidden="true" />
            </span>
            <div className="text-kn-sm font-kn-bold kn-text-primary">Nothing queued yet</div>
            <div className="text-kn-xs kn-text-muted">Pick a file, folder, clipboard, or note above</div>
          </div>
        ) : (
          <>
            <div className="mb-2.5 flex items-center justify-between px-0.5">
              <span className="text-kn-sm font-kn-bold tabular-nums">
                {joinMeta(pluralize(sendQueue.length, 'item'), formatBytes(totalBytes))}
              </span>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setShareSheetOpen(true)}
                  aria-label="Share queued items"
                  className={`flex h-11 w-11 cursor-pointer items-center justify-center rounded-kn-xl transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) ${FOCUS_RING} kn-surface-tonal kn-surface-hover kn-text-secondary`}
                >
                  <Share2 className="h-5 w-5" aria-hidden="true" />
                </button>
                <button
                  type="button"
                  onClick={() => setSendQueue([])}
                  className={`min-h-(--kn-size-44px) cursor-pointer rounded-kn-xl px-3.5 text-kn-xs font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) ${FOCUS_RING} kn-surface-tonal kn-surface-hover kn-text-secondary`}
                >
                  Clear all
                </button>
              </div>
            </div>

            <div className="space-y-2">
              {sendQueue.map((item) => (
                <div
                  key={item.id}
                  className="animate-kn-fade-in flex min-h-(--kn-size-54px) items-center justify-between gap-2.5 rounded-kn-xl px-3.5 py-2.5 kn-surface-tonal"
                >
                  <div className="flex min-w-0 items-center gap-3">
                    {item.isDirectory ? (
                      <Folder className="h-5 w-5 shrink-0 kn-loading-text" aria-hidden="true" />
                    ) : (
                      <FileText className="h-5 w-5 shrink-0 kn-loading-text" aria-hidden="true" />
                    )}
                    <span className="truncate text-kn-sm">{item.name}</span>
                  </div>
                  <div className="flex shrink-0 items-center gap-2.5">
                    <span className="text-kn-xs tabular-nums kn-text-muted">
                      {formatBytes(item.sizeBytes)}
                    </span>
                    <button
                      type="button"
                      onClick={() => setSendQueue((prev) => prev.filter((q) => q.id !== item.id))}
                      aria-label={`Remove ${item.name}`}
                      className={`flex h-11 w-11 cursor-pointer items-center justify-center rounded-kn-xl transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) ${FOCUS_RING} kn-surface kn-surface-hover kn-text-secondary`}
                    >
                      <X className="h-5 w-5" aria-hidden="true" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
      </Card>

      {outgoingTransfers.length > 0 && (
        <Card className="space-y-4">
          {outgoingTransfers.map((tx) => {
            const pct =
              tx.totalBytes > 0
                ? Math.min(100, Math.round((tx.transferredBytes / tx.totalBytes) * 100))
                : 0;
            const first = tx.items[0]?.name ?? 'Files';
            const summary = tx.items.length <= 1 ? first : `${first} +${tx.items.length - 1}`;
            return (
              <div key={tx.transferId} className="space-y-3">
                <div className="flex items-baseline justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-kn-base font-kn-semibold">{summary}</div>
                    <div className="mt-0.5 truncate text-kn-xs tabular-nums kn-text-muted">
                      {joinMeta(`To ${tx.peerName}`, formatBytes(tx.totalBytes))}
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
                    aria-label={`Transfer progress for ${summary}`}
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

      <section>
        <SectionHeading>Send to</SectionHeading>
        <ListGroup>
          {[
            ...pairedDevices.map((d) => ({
              id: d.deviceId,
              displayName: d.displayName,
              subtitle: d.connectionLabel,
            })),
            ...nearbyPeers.map((p) => ({
              id: p.presenceToken,
              displayName: p.displayName,
              subtitle: joinMeta('Nearby', `${p.rssi} dBm`),
            })),
          ].map((target, idx) => (
            <div
              key={target.id}
              className="flex min-h-(--kn-size-68px) items-center justify-between gap-3 rounded-kn-xl px-4 py-3"
            >
              <div className="flex min-w-0 items-center gap-3">
                <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-kn-xl kn-loading-tonal">
                  <Laptop className="h-5 w-5" aria-hidden="true" />
                </div>
                <div className="min-w-0">
                  <div className="truncate text-kn-base font-kn-semibold">{target.displayName}</div>
                  <div className="truncate text-kn-xs kn-text-muted">{target.subtitle}</div>
                </div>
              </div>
              <button
                type="button"
                onClick={() =>
                  startOutgoingTransfer(
                    { deviceId: target.id, displayName: target.displayName },
                    sendQueue
                  )
                }
                className={`inline-flex min-h-(--kn-size-48px) shrink-0 items-center gap-2 rounded-kn-xl px-5 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-150) active:scale-(--kn-press-95) ${FOCUS_RING} ${
                  idx === 0 ? 'kn-loading-bg' : 'kn-loading-tonal'
                }`}
              >
                <Send className="h-4 w-4" aria-hidden="true" />
                <span>Send</span>
              </button>
            </div>
          ))}
        </ListGroup>
      </section>

      {showTextComposer && (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="add-text-dialog-title"
          onClick={(e) => {
            if (e.target === e.currentTarget) setShowTextComposer(false);
          }}
          className="animate-kn-fade-in absolute inset-0 z-50 flex items-center justify-center p-4 kn-scrim"
        >
          <form
            onSubmit={handleAddText}
            className="animate-kn-modal-in w-full space-y-3.5 rounded-kn-2xl p-4 kn-surface"
          >
            <h2 id="add-text-dialog-title" className="text-kn-lg font-kn-bold">
              Add text
            </h2>
            <div>
              <label htmlFor="send-note-input" className="sr-only">
                Paste URL or note
              </label>
              <input
                id="send-note-input"
                type="text"
                autoFocus
                value={customTextSnippet}
                onChange={(e) => setCustomTextSnippet(e.target.value)}
                placeholder="Paste URL or note"
                className={`min-h-(--kn-size-52px) w-full rounded-kn-xl px-4 text-kn-base ${FOCUS_RING} kn-input`}
              />
            </div>
            <div className="flex gap-2.5">
              <button
                type="button"
                onClick={() => {
                  setCustomTextSnippet('');
                  setShowTextComposer(false);
                }}
                className={`min-h-(--kn-size-48px) flex-1 rounded-kn-xl px-4 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) ${FOCUS_RING} kn-surface-tonal`}
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={!customTextSnippet.trim()}
                className={`min-h-(--kn-size-48px) flex-1 rounded-kn-xl px-4 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) disabled:opacity-50 ${FOCUS_RING} kn-loading-bg`}
              >
                Add
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}
