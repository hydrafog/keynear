import { useRef, useState, type KeyboardEvent, type PointerEvent } from 'react';
import { useApp } from '../../state/AppProvider';
import { orbitFor } from '../../lib/orbit';
import type { VisibilityMode } from '../../types';
import { WaveMapCanvas } from './WaveMapCanvas';
import { ReceiveBanners } from './ReceiveBanners';
import { ShieldCheck, Laptop, ChevronRight, AlertTriangle, Radio, EyeOff, Smartphone } from 'lucide-react';

const VISIBILITY_MODES = [
  { id: 'hidden', label: 'Hidden', icon: EyeOff },
  { id: 'pairedOnly', label: 'Paired', icon: ShieldCheck },
  { id: 'everyoneNearby', label: 'Everyone', icon: Radio },
] as const;

const DRAG_PX = 6;
const SHEET_COLLAPSED_PX = 132;
const SHEET_EXPANDED_PX = 240;
const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500';

export function ReceiveScreen() {
  const {
    darkMode, setSubScreen, config, everyoneNearbySecondsLeft, showAllDevicesSheet,
    setShowAllDevicesSheet, missingPermissionsCount, formatCountdown, handleVisibilityChange,
    triggerBlePairing, openDeviceDetail, mapPins, visibilityIndex,
  } = useApp();

  const [dragPillIdx, setDragPillIdx] = useState<number | null>(null);
  const segTrackRef = useRef<HTMLDivElement | null>(null);
  const segDrag = useRef({ active: false, moved: false, id: -1, x: 0, base: 0, last: 0 });
  const collapsedH = SHEET_COLLAPSED_PX;
  const expandedH = SHEET_EXPANDED_PX;
  const [dragSheetH, setDragSheetH] = useState<number | null>(null);
  const sheetDrag = useRef({ active: false, moved: false, id: -1, y: 0, startH: 0 });

  const onSegDown = (e: PointerEvent<HTMLDivElement>) => {
    const base = Math.max(0, visibilityIndex);
    segDrag.current = { active: true, moved: false, id: e.pointerId, x: e.clientX, base, last: base };
  };
  const onSegMove = (e: PointerEvent<HTMLDivElement>) => {
    const st = segDrag.current;
    if (!st.active || st.id !== e.pointerId) return;
    const dx = e.clientX - st.x;
    if (!st.moved && Math.abs(dx) < DRAG_PX) return;
    if (!st.moved) {
      st.moved = true;
      e.currentTarget.setPointerCapture(e.pointerId);
    }
    const slotW = (segTrackRef.current?.getBoundingClientRect().width || 300) / VISIBILITY_MODES.length;
    const raw = Math.max(0, Math.min(VISIBILITY_MODES.length - 1, st.base + dx / slotW));
    setDragPillIdx(raw);
    const snapped = Math.round(raw);
    if (snapped !== st.last) {
      st.last = snapped;
      handleVisibilityChange(VISIBILITY_MODES[snapped].id);
    }
  };
  const onSegEnd = (e: PointerEvent<HTMLDivElement>) => {
    const st = segDrag.current;
    if (!st.active || st.id !== e.pointerId) return;
    if (st.moved && e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
    st.active = false;
    setDragPillIdx(null);
  };
  const onSegKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
    e.preventDefault();
    const next = Math.max(0, Math.min(VISIBILITY_MODES.length - 1, Math.max(0, visibilityIndex) + (e.key === 'ArrowRight' ? 1 : -1)));
    handleVisibilityChange(VISIBILITY_MODES[next].id);
  };

  const onSheetDown = (e: PointerEvent<HTMLButtonElement>) => {
    sheetDrag.current = { active: true, moved: false, id: e.pointerId, y: e.clientY, startH: showAllDevicesSheet ? expandedH : collapsedH };
  };
  const onSheetMove = (e: PointerEvent<HTMLButtonElement>) => {
    const st = sheetDrag.current;
    if (!st.active || st.id !== e.pointerId) return;
    const dy = e.clientY - st.y;
    if (!st.moved && Math.abs(dy) < DRAG_PX) return;
    if (!st.moved) {
      st.moved = true;
      e.currentTarget.setPointerCapture(e.pointerId);
    }
    setDragSheetH(Math.max(0, Math.min(expandedH, st.startH - dy)));
  };
  const onSheetEnd = (e: PointerEvent<HTMLButtonElement>) => {
    const st = sheetDrag.current;
    if (!st.active || st.id !== e.pointerId) return;
    if (st.moved && e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
    if (st.moved && dragSheetH !== null) setShowAllDevicesSheet(dragSheetH >= (collapsedH + expandedH) / 2);
    st.active = false;
    setDragSheetH(null);
  };

  const pillPos = dragPillIdx ?? Math.max(0, visibilityIndex);
  const sheetHeight = dragSheetH !== null ? dragSheetH : showAllDevicesSheet ? expandedH : collapsedH;
  const radarActive = config.visibilityMode !== 'hidden';

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="relative flex-1 overflow-hidden">
        <div className="pointer-events-none absolute inset-0 pt-4">
          <WaveMapCanvas active={radarActive} darkMode={darkMode} />
        </div>
        <div
          aria-hidden="true"
          className={`pointer-events-none absolute inset-0 flex items-center justify-center pt-4 transition-opacity duration-(--kn-duration-500) ${radarActive ? 'opacity-100' : 'opacity-0'}`}
        >
          <div className="h-(--kn-size-290px) w-(--kn-size-290px) rounded-full kn-brand-aura" />
        </div>

        <div className="absolute left-3 right-3 top-3 z-20 space-y-2.5">
          {missingPermissionsCount > 0 && (
            <button
              type="button"
              onClick={() => setSubScreen('permissions')}
              className={`animate-kn-fade-in flex min-h-(--kn-size-52px) w-full items-center justify-between rounded-kn-2xl px-4 py-3 text-left transition-all duration-(--kn-duration-150) active:scale-(--kn-press-98) ${FOCUS_RING} kn-error-tonal`}
            >
              <span className="flex items-center gap-2.5 text-kn-sm font-kn-bold">
                <AlertTriangle className="h-5 w-5 shrink-0" aria-hidden="true" />
                <span>{missingPermissionsCount} Android {missingPermissionsCount === 1 ? 'permission' : 'permissions'} required</span>
              </span>
              <ChevronRight className="h-5 w-5 shrink-0" aria-hidden="true" />
            </button>
          )}

          <div className="rounded-kn-2xl p-1.5 kn-fog-well">
            <div
              ref={segTrackRef}
              role="group"
              aria-label="Discovery visibility"
              onKeyDown={onSegKey}
              onPointerDown={onSegDown}
              onPointerMove={onSegMove}
              onPointerUp={onSegEnd}
              onPointerCancel={onSegEnd}
              className="relative grid grid-cols-3 touch-pan-y select-none"
            >
              <div
                aria-hidden="true"
                style={{ transform: `translateX(${pillPos * 100}%)` }}
                className={`pointer-events-none absolute inset-y-0 left-0 w-1/3 rounded-kn-xl kn-loading-bg ${dragPillIdx === null ? 'transition-transform duration-(--kn-duration-morph) ease-(--ease-kn-spring)' : ''}`}
              />
              {VISIBILITY_MODES.map(({ id, label, icon: Icon }) => {
                const active = config.visibilityMode === id;
                return (
                  <button
                    key={id}
                    type="button"
                    aria-pressed={active}
                    onClick={() => {
                      if (segDrag.current.moved) {
                        segDrag.current.moved = false;
                        return;
                      }
                      handleVisibilityChange(id as VisibilityMode);
                    }}
                    className={`relative z-10 flex min-h-(--kn-size-48px) items-center justify-center gap-2 rounded-kn-xl px-2.5 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-150) ${FOCUS_RING} ${active ? 'text-kn-white' : 'kn-text-secondary'}`}
                  >
                    <Icon className="h-4 w-4 shrink-0" aria-hidden="true" />
                    <span>{label}</span>
                  </button>
                );
              })}
            </div>
          </div>
          <ReceiveBanners />
        </div>

        <div className="pointer-events-none absolute inset-0 flex items-center justify-center pt-4">
          <div className="flex flex-col items-center">
            <div className={`flex h-16 w-16 items-center justify-center rounded-full transition-all duration-(--kn-duration-morph) ease-(--ease-kn-spring) ${radarActive ? 'kn-loading-bg' : 'kn-fog-surface'}`}>
              <Smartphone className="h-7 w-7" aria-hidden="true" />
            </div>
            <span className="mt-2.5 rounded-kn-xl px-3.5 py-1.5 text-kn-xs font-kn-bold kn-fog-surface">
              Pixel 9 Pro
              {config.visibilityMode === 'everyoneNearby' && (
                <span className="ml-1.5 tabular-nums kn-loading-text">{formatCountdown(everyoneNearbySecondsLeft)}</span>
              )}
            </span>
          </div>
        </div>

        <div className="pointer-events-none absolute inset-0 flex items-center justify-center pt-4">
          <div className="relative h-0 w-0">
            {mapPins.map((pin, idx) => {
              const orbit = orbitFor(pin.rssi, idx, mapPins.length);
              return (
                <div
                  key={pin.id}
                  className="animate-kn-orbit absolute left-0 top-0 h-0 w-0"
                  style={{ animationDuration: `${orbit.duration}s`, animationDelay: `-${orbit.phase}s` }}
                >
                  <div className="absolute left-0 top-0 h-0 w-0" style={{ translate: `${orbit.radius}px 0` }}>
                    <button
                      type="button"
                      onClick={() => (pin.isPaired && pin.rawDevice ? openDeviceDetail(pin.rawDevice) : triggerBlePairing(pin.displayName))}
                      style={{ animationDuration: `${orbit.duration}s`, animationDelay: `-${orbit.phase}s` }}
                      className={`animate-kn-orbit-counter pointer-events-auto group absolute left-0 top-0 z-10 flex min-h-(--kn-size-56px) min-w-(--kn-size-56px) -translate-x-1/2 -translate-y-1/2 flex-col items-center rounded-kn-2xl text-center ${FOCUS_RING}`}
                    >
                      <div className="animate-kn-pin-enter flex flex-col items-center" style={{ animationDelay: `${idx * 55}ms` }}>
                        <div className="relative flex h-14 w-14 items-center justify-center rounded-full transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) kn-fog-surface kn-surface-hover">
                          <Laptop className="h-6 w-6 kn-loading-text" aria-hidden="true" />
                        </div>
                        <div className="mt-2 rounded-kn-xl px-3.5 py-1.5 kn-fog-surface">
                          <div className="max-w-(--kn-size-136px) truncate text-kn-xs font-kn-bold leading-kn-tight">{pin.displayName}</div>
                          <div className="mt-0.5 text-kn-xs leading-kn-tight kn-text-muted">{pin.state}</div>
                          <div className="text-kn-xs leading-kn-tight tabular-nums kn-text-muted">{pin.rssi} dBm</div>
                        </div>
                      </div>
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      <section aria-label="Devices in range" className="mx-4 shrink-0 rounded-t-3xl px-4 pb-3 pt-2.5 kn-fog-sheet">
        {mapPins.length === 0 ? (
          <div className="flex flex-col items-center">
            <span aria-hidden="true" className="h-1.5 w-12 rounded-full kn-sheet-handle" />
            <div className="flex flex-col items-center gap-2 px-3 py-4 text-center">
              <span className="flex h-11 w-11 items-center justify-center rounded-kn-xl kn-loading-tonal">
                <Radio className="h-5 w-5" aria-hidden="true" />
              </span>
              <div className="text-kn-sm font-kn-bold kn-text-primary">No computers nearby</div>
              <div className="text-kn-xs kn-text-muted">Switch visibility to Paired or Everyone</div>
            </div>
          </div>
        ) : (
          <>
            <button
              type="button"
              onPointerDown={onSheetDown}
              onPointerMove={onSheetMove}
              onPointerUp={onSheetEnd}
              onPointerCancel={onSheetEnd}
              onClick={() => {
                if (sheetDrag.current.moved) {
                  sheetDrag.current.moved = false;
                  return;
                }
                setShowAllDevicesSheet(!showAllDevicesSheet);
              }}
              aria-expanded={showAllDevicesSheet}
              className={`flex min-h-(--kn-size-44px) w-full touch-none flex-col items-center justify-center rounded-kn-xl pb-1.5 pt-0.5 transition-opacity duration-(--kn-duration-150) active:opacity-75 ${FOCUS_RING}`}
            >
              <span aria-hidden="true" className="h-1.5 w-12 rounded-full kn-sheet-handle" />
              <div className="mt-2.5 flex w-full items-center justify-between">
                <span className="text-kn-sm font-kn-bold">
                  {mapPins.length} {mapPins.length === 1 ? 'computer' : 'computers'} in range
                </span>
                <span className="text-kn-sm font-kn-bold kn-loading-text">{showAllDevicesSheet ? 'Collapse' : 'All devices'}</span>
              </div>
            </button>

            <div
              style={{ maxHeight: `${sheetHeight}px` }}
              className={`space-y-1.5 overflow-y-auto kn-scroll-hidden ${dragSheetH === null ? 'transition-[max-height] duration-(--kn-duration-morph) ease-(--ease-kn-spring)' : ''}`}
            >
              {mapPins.map((pin) => (
                <button
                  key={pin.id}
                  type="button"
                  onClick={() => (pin.isPaired && pin.rawDevice ? openDeviceDetail(pin.rawDevice) : triggerBlePairing(pin.displayName))}
                  className={`flex min-h-(--kn-size-62px) w-full items-center justify-between gap-3.5 rounded-kn-xl px-3.5 py-2.5 text-left transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-99) ${FOCUS_RING} kn-surface-hover`}
                >
                  <div className="flex min-w-0 items-center gap-3.5">
                    <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-kn-xl kn-loading-tonal">
                      <Laptop className="h-5 w-5" aria-hidden="true" />
                    </div>
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="truncate text-kn-base font-kn-bold">{pin.displayName}</span>
                        {!pin.isPaired && <span className="rounded-kn-lg px-2.5 py-0.5 text-kn-xs font-kn-bold kn-loading-tonal">Pair</span>}
                      </div>
                      <div className="truncate text-kn-xs tabular-nums kn-text-muted">{pin.state} {pin.rssi} dBm</div>
                    </div>
                  </div>
                  <ChevronRight className="h-5 w-5 shrink-0 kn-text-muted" aria-hidden="true" />
                </button>
              ))}
            </div>
          </>
        )}
      </section>
    </div>
  );
}
