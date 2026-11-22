import { useApp } from '../../state/AppProvider';

export function Snackbar() {
  const { snackbarMessage } = useApp();
  return (
          <div
            role="status"
            aria-live="polite"
            className="animate-kn-sheet-up pointer-events-none absolute bottom-28 left-3 right-3 z-30 flex justify-center"
          >
            <div className={`rounded-kn-xl px-5 py-3 text-kn-sm font-kn-semibold kn-snackbar`}>
              {snackbarMessage}
            </div>
          </div>
  );
}
