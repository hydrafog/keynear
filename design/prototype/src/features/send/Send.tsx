import { useEffect, type FormEvent } from 'react';
import { useApp } from '../../state/AppProvider';
import { SelectionTile } from '../../design-system/Controls';
import { SectionHeading } from '../../design-system/Setting';
import { Laptop, X, Folder, FileText, Clipboard, Send, Plus } from 'lucide-react';

const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500';

export function SendScreen() {
  const {
    pairedDevices,
    nearbyPeers,
    sendQueue,
    setSendQueue,
    showTextComposer,
    setShowTextComposer,
    customTextSnippet,
    setCustomTextSnippet,
    formatBytes,
    startOutgoingTransfer,
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

      <section className="rounded-kn-2xl p-4 kn-surface">
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
                {sendQueue.length} {sendQueue.length === 1 ? 'item' : 'items'} · {formatBytes(totalBytes)}
              </span>
              <button
                type="button"
                onClick={() => setSendQueue([])}
                className={`min-h-(--kn-size-38px) cursor-pointer rounded-kn-xl px-3.5 text-kn-xs font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) ${FOCUS_RING} kn-surface-tonal kn-surface-hover kn-text-secondary`}
              >
                Clear all
              </button>
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
                      className={`flex h-10 w-10 cursor-pointer items-center justify-center rounded-kn-xl transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) ${FOCUS_RING} kn-surface kn-surface-hover kn-text-secondary`}
                    >
                      <X className="h-5 w-5" aria-hidden="true" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
      </section>

      <section>
        <SectionHeading>Send to</SectionHeading>
        <div className="space-y-1.5 rounded-kn-2xl p-2 kn-surface">
          {[
            ...pairedDevices.map((d) => ({
              id: d.deviceId,
              displayName: d.displayName,
              subtitle: d.connectionLabel,
            })),
            ...nearbyPeers.map((p) => ({
              id: p.presenceToken,
              displayName: p.displayName,
              subtitle: `Nearby (${p.rssi} dBm)`,
            })),
          ].map((target, idx) => (
            <div
              key={target.id}
              className="flex min-h-(--kn-size-68px) items-center justify-between gap-3 rounded-kn-xl px-3.5 py-3"
            >
              <div className="flex min-w-0 items-center gap-3.5">
                <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-kn-xl kn-loading-tonal">
                  <Laptop className="h-6 w-6" aria-hidden="true" />
                </div>
                <div className="min-w-0">
                  <div className="truncate text-kn-base font-kn-bold">{target.displayName}</div>
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
        </div>
      </section>

      {showTextComposer && (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="add-text-dialog-title"
          onClick={(e) => {
            if (e.target === e.currentTarget) setShowTextComposer(false);
          }}
          className="animate-kn-fade-in absolute inset-0 z-50 flex items-center justify-center p-5 kn-scrim"
        >
          <form
            onSubmit={handleAddText}
            className="animate-kn-modal-in w-full space-y-3.5 rounded-kn-2xl p-5 kn-surface"
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
