export function Snackbar({ message, closing }: { message: string | null; closing: boolean }) {
  return (
          <div
            role="status"
            aria-live="polite"
            className={`${closing ? 'animate-kn-sheet-down' : 'animate-kn-sheet-up'} pointer-events-none absolute bottom-28 left-4 right-4 z-30 flex justify-center`}
          >
            <div className={`rounded-kn-xl px-5 py-3 text-kn-sm font-kn-semibold kn-snackbar`}>
              {message}
            </div>
          </div>
  );
}
