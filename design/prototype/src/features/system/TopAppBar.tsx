import { useApp } from '../../state/AppProvider';
import { ArrowLeft } from 'lucide-react';

export function TopAppBar() {
  const { activeTab, subScreen, setSubScreen, selectedDevice } = useApp();
  return (
    <header className="flex h-16 shrink-0 items-center justify-between px-4">
      <div className="flex min-w-0 items-center gap-2.5">
        {subScreen && (
          <button
            type="button"
            onClick={() => setSubScreen(null)}
            aria-label="Navigate back"
            className="flex h-12 w-12 items-center justify-center rounded-kn-2xl transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus kn-surface"
          >
            <ArrowLeft className="h-6 w-6" aria-hidden="true" />
          </button>
        )}
        <h1 className="truncate text-kn-2xl font-kn-bold leading-kn-tight tracking-kn-tight">
          {subScreen === 'deviceDetail'
            ? selectedDevice?.displayName || 'Device details'
            : subScreen === 'permissions'
            ? 'System permissions'
            : activeTab === 'receive'
            ? 'Keynear'
            : activeTab === 'send'
            ? 'Send files'
            : 'Settings'}
        </h1>
      </div>
    </header>
  );
}
