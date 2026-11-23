import { useApp } from '../../state/AppProvider';
import { usePresence } from '../../lib/usePresence';
import { StatusBar } from './StatusBar';
import { TopAppBar } from './TopAppBar';
import { BottomNav } from './BottomNav';
import { Snackbar } from './Snackbar';
import { ShareSheet } from './ShareSheet';
import { LockScreen } from './LockScreen';
import { Shade } from '../shade/Shade';
import { DeviceDetail } from '../devices/DeviceDetail';
import { Permissions } from '../settings/Permissions';
import { ReceiveScreen } from '../receive/Receive';
import { SendScreen } from '../send/Send';
import { SettingsScreen } from '../settings/Settings';

export function AppShell() {
  const { activeTab, subScreen, shadeOpen, shareSheetOpen, lockScreenOpen, snackbarMessage, selectedDevice } = useApp();
  // Overlays stay mounted briefly after closing so they can play their exit animation.
  const shade = usePresence(shadeOpen, 220);
  const share = usePresence(shareSheetOpen, 220);
  const lock = usePresence(lockScreenOpen, 180);
  const snack = usePresence(snackbarMessage, 220);
  return (
    <div className="min-h-(--kn-size-100svh) w-full flex justify-center transition-colors duration-(--kn-duration-200) kn-backdrop">
      <div className="relative flex h-(--kn-size-100svh) w-full max-w-(--kn-size-430px) flex-col overflow-hidden isolate transition-colors duration-(--kn-duration-200) kn-canvas">
        <StatusBar />

        <TopAppBar />

        <main
          key={subScreen || activeTab}
          className={`animate-kn-screen-in flex-1 ${
            !subScreen && activeTab === 'receive'
              ? 'flex flex-col overflow-hidden'
              : 'overflow-y-auto px-4 pb-6 pt-1'
          }`}
        >

          {subScreen === 'deviceDetail' && selectedDevice ? (
            <DeviceDetail key="deviceDetail" />
          ) : subScreen === 'permissions' ? (

            <Permissions key="permissions" />
          ) : activeTab === 'receive' ? (

            <ReceiveScreen key="receive" />
          ) : activeTab === 'send' ? (

            <SendScreen key="send" />
          ) : (

            <SettingsScreen key="settings" />
          )}
        </main>

        {snack.mounted && <Snackbar message={snack.value} closing={snack.closing} />}

        <BottomNav />

        {shade.mounted && <Shade closing={shade.closing} />}

        {share.mounted && <ShareSheet closing={share.closing} />}

        {lock.mounted && <LockScreen closing={lock.closing} />}
      </div>
    </div>
  );
}
