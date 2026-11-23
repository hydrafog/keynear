import { useApp } from '../../state/AppProvider';
import { ListGroup } from '../../design-system/Card';
import type { PermissionKey } from '../../types';

interface PermissionRowSpec {
  key: PermissionKey;
  title: string;
  desc: string;
}

const PERMISSION_ROWS: PermissionRowSpec[] = [
  {
    key: 'nearbyDevices',
    title: 'Nearby devices',
    desc: 'Bluetooth and Wi-Fi discovery for nearby Linux desktops.',
  },
  {
    key: 'localNetwork',
    title: 'Local network access',
    desc: 'Direct peer-to-peer transfers over your local Wi-Fi.',
  },
  {
    key: 'notifications',
    title: 'Notifications',
    desc: 'Shows transfer requests, 6-digit pairing codes, and live progress.',
  },
  {
    key: 'batteryUnrestricted',
    title: 'Unrestricted battery',
    desc: 'Prevents Android from interrupting large transfers with the screen off.',
  },
];

export function Permissions() {
  const { permissions, setPermissions, showSnackbar } = useApp();
  return (
    <div className="space-y-4">
      <ListGroup>
        {PERMISSION_ROWS.map((perm) => {
          const granted = permissions[perm.key];
          return (
            <div key={perm.key} className="flex items-center justify-between gap-3 rounded-kn-xl px-4 py-3">
              <div className="min-w-0">
                <div className="text-kn-base font-kn-semibold">{perm.title}</div>
                <div className="mt-1 text-kn-xs leading-kn-relaxed kn-text-muted">
                  {perm.desc}
                </div>
              </div>
              <button
                type="button"
                onClick={() => {
                  setPermissions((p) => ({
                    ...p,
                    [perm.key]: !p[perm.key],
                  }));
                  showSnackbar(!granted ? `Allowed ${perm.title}.` : `Revoked ${perm.title}.`);
                }}
                className={`min-h-(--kn-size-48px) shrink-0 cursor-pointer rounded-kn-xl px-5 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus ${
                  granted ? 'kn-loading-tonal' : 'kn-loading-bg'
                }`}
              >
                {granted ? 'Allowed' : 'Allow'}
              </button>
            </div>
          );
        })}
      </ListGroup>
    </div>
  );
}
