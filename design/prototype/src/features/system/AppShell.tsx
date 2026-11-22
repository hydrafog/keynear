import { useApp } from '../../state/AppProvider';
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

        {snackbarMessage && (
          <Snackbar />
        )}

        <BottomNav />

        {shadeOpen && (
          <Shade />
        )}

        {shareSheetOpen && (
          <ShareSheet />
        )}

        {lockScreenOpen && (
          <LockScreen />
        )}
      </div>
    </div>
  );
}
