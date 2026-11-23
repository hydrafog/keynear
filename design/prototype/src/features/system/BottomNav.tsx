import { useApp } from '../../state/AppProvider';
import { Send, SlidersHorizontal, Inbox } from 'lucide-react';

export function BottomNav() {
  const { activeTab, setActiveTab, subScreen, setSubScreen, missingPermissionsCount } = useApp();
  return (
        <footer className="shrink-0">
          <nav aria-label="Android app navigation" className="grid grid-cols-3 px-4 pt-2 pb-1.5">
            {[
              { id: 'receive' as const, label: 'Receive', icon: Inbox },
              { id: 'send' as const, label: 'Send', icon: Send },
              { id: 'settings' as const, label: 'Settings', icon: SlidersHorizontal, badge: missingPermissionsCount },
            ].map((tab) => {
              const Icon = tab.icon;
              const active = activeTab === tab.id;
              return (
                <button
                  key={tab.id}
                  type="button"
                  aria-current={active ? (subScreen ? 'true' : 'page') : undefined}
                  onClick={() => {
                    setSubScreen(null);
                    setActiveTab(tab.id);
                  }}
                  className="flex min-h-(--kn-size-58px) flex-col items-center justify-center gap-1.5 rounded-kn-xl transition-transform duration-(--kn-duration-150) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus"
                >
                  <span
                    className={`relative flex h-9 items-center justify-center rounded-full transition-all duration-(--kn-duration-morph) ease-(--ease-kn-spring) ${
                      active ? 'w-16 kn-loading-tonal' : 'w-10 bg-transparent'
                    }`}
                  >
                    <Icon className="h-5 w-5" aria-hidden="true" />
                    {(tab.badge ?? 0) > 0 && (
                      <span className="absolute right-2 top-1.5 h-2.5 w-2.5 rounded-full kn-error-bg" />
                    )}
                  </span>
                  <span
                    className={`text-kn-xs font-kn-bold transition-colors duration-(--kn-duration-150) ${
                      active ? 'kn-text-primary' : 'kn-text-muted'
                    }`}
                  >
                    {tab.label}
                  </span>
                </button>
              );
            })}
          </nav>

          <div className="flex justify-center pb-2.5 pt-0.5">
            <div
              aria-hidden="true"
              className={`h-1.5 w-32 rounded-full kn-gesture-pill`}
            />
          </div>
        </footer>
  );
}
