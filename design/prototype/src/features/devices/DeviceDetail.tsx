import { useApp } from '../../state/AppProvider';
import { Laptop, RefreshCw, Trash2 } from 'lucide-react';
import { Switch } from '../../design-system/Switch';
import { Card } from '../../design-system/Card';
import { joinMeta } from '../../lib/format';

const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus';

export function DeviceDetail() {
  const { editingName, setEditingName, confirmUnpairId, setConfirmUnpairId, showSnackbar, selectedDevice, saveDeviceName, togglePerDeviceAutoAccept, unpairDevice, renderStamp } = useApp();
  if (!selectedDevice) return null;
  return (
    <div className="space-y-4">
      <Card className="flex items-center gap-4">
        <div className="flex h-14 w-14 shrink-0 items-center justify-center rounded-kn-xl kn-loading-tonal">
          <Laptop className="h-7 w-7" aria-hidden="true" />
        </div>
        <div className="min-w-0">
          <div className="truncate text-kn-lg font-kn-bold">{selectedDevice.displayName}</div>
          <div className="text-kn-sm kn-text-muted">
            {joinMeta(selectedDevice.connectionLabel, `Paired ${selectedDevice.pairedAt}`)}
          </div>
        </div>
      </Card>

      <Card label="Device details" className="space-y-5">
        <form onSubmit={saveDeviceName}>
          <label
            htmlFor="device-name-input"
            className="block text-kn-sm font-kn-semibold kn-text-secondary"
          >
            Computer name
          </label>
          <div className="mt-2 flex gap-2.5">
            <input
              id="device-name-input"
              type="text"
              value={editingName}
              onChange={(e) => setEditingName(e.target.value)}
              className={`min-h-(--kn-size-52px) min-w-0 flex-1 rounded-kn-xl px-4 text-kn-base ${FOCUS_RING} kn-input`}
            />
            <button
              type="submit"
              className={`min-h-(--kn-size-52px) rounded-kn-xl px-5 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-150) active:scale-(--kn-press-95) ${FOCUS_RING} kn-loading-bg`}
            >
              Save
            </button>
          </div>
        </form>

        <div className="flex items-center justify-between gap-3">
          <div className="pr-2">
            <div className="text-kn-base font-kn-semibold">Auto-accept files</div>
            <div className="text-kn-xs kn-text-muted">
              Save files from {selectedDevice.displayName} without asking
            </div>
          </div>
          <Switch
            key={renderStamp}
            checked={selectedDevice.autoAcceptFiles}
            label="Auto-accept files from this device"
            onChange={() => togglePerDeviceAutoAccept(selectedDevice.deviceId)}
          />
        </div>

        <dl className="space-y-3.5 pt-1 text-kn-sm">
          <div className="flex items-center justify-between gap-2">
            <dt className="kn-text-muted">Key fingerprint</dt>
            <dd className="min-w-0 break-all text-right text-kn-xs font-kn-mono">
              {selectedDevice.staticX25519Pubkey.slice(0, 19)}…
            </dd>
          </div>
          <div className="flex items-center justify-between gap-2">
            <dt className="kn-text-muted">Bluetooth address</dt>
            <dd className="min-w-0 break-all text-right text-kn-xs font-kn-mono">{selectedDevice.lastBleAddress}</dd>
          </div>
          <div className="flex items-center justify-between gap-2">
            <dt className="kn-text-muted">Local Wi-Fi endpoint</dt>
            <dd className="min-w-0 break-all text-right text-kn-xs font-kn-mono">{selectedDevice.lastLanAddress}</dd>
          </div>
        </dl>
      </Card>

      <div className="space-y-3">
        <button
          type="button"
          onClick={() => showSnackbar(`Cleared Bluetooth bond for ${selectedDevice.displayName}.`)}
          className={`flex min-h-(--kn-size-54px) w-full items-center justify-center gap-2.5 rounded-kn-xl px-5 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-98) ${FOCUS_RING} kn-surface`}
        >
          <RefreshCw className="h-5 w-5" aria-hidden="true" />
          <span>Reset Bluetooth bond</span>
        </button>

        {confirmUnpairId === selectedDevice.deviceId ? (
          <Card tone="tonal" className="animate-kn-modal-in">
            <p className="text-kn-sm font-kn-semibold leading-kn-relaxed kn-text-primary">
              Remove {selectedDevice.displayName}? You will need to confirm a new 6-digit code on
              both devices to pair again.
            </p>
            <div className="mt-4 flex gap-3">
              <button
                type="button"
                onClick={() => unpairDevice(selectedDevice.deviceId)}
                className={`min-h-(--kn-size-52px) flex-1 cursor-pointer rounded-kn-xl px-4 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) ${FOCUS_RING} kn-error-bg`}
              >
                Remove device
              </button>
              <button
                type="button"
                onClick={() => setConfirmUnpairId(null)}
                className={`min-h-(--kn-size-52px) flex-1 cursor-pointer rounded-kn-xl px-4 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) ${FOCUS_RING} kn-surface kn-surface-hover`}
              >
                Cancel
              </button>
            </div>
          </Card>
        ) : (
          <button
            type="button"
            onClick={() => setConfirmUnpairId(selectedDevice.deviceId)}
            className={`flex min-h-(--kn-size-54px) w-full cursor-pointer items-center justify-center gap-2.5 rounded-kn-xl px-5 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-98) ${FOCUS_RING} kn-surface-tonal kn-surface-hover kn-text-secondary`}
          >
            <Trash2 className="h-5 w-5" aria-hidden="true" />
            <span>Remove device</span>
          </button>
        )}
      </div>
    </div>
  );
}
