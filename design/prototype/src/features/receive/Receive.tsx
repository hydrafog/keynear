import { useRef, useState, type KeyboardEvent, type PointerEvent } from 'react';
import { useApp } from '../../state/AppProvider';
import { orbitFor, radarLayoutFor } from '../../lib/orbit';
import { joinMeta, pluralize, VISIBILITY_LABELS } from '../../lib/format';
import { useElementSize } from '../../lib/useElementSize';
import { useOrbitRate } from '../../lib/useOrbitRate';
import type { VisibilityMode } from '../../types';
import { WaveMapCanvas } from './WaveMapCanvas';
import { ReceiveBanners } from './ReceiveBanners';
import { ShieldCheck, Laptop, ChevronRight, AlertTriangle, Radio, EyeOff, Smartphone } from 'lucide-react';

// The three-up control is too narrow for "Everyone nearby", so it uses the short labels.
const VISIBILITY_MODES = [
  { id: 'hidden', label: VISIBILITY_LABELS.hidden.short, icon: EyeOff },
  { id: 'pairedOnly', label: VISIBILITY_LABELS.pairedOnly.short, icon: ShieldCheck },
  { id: 'everyoneNearby', label: VISIBILITY_LABELS.everyoneNearby.short, icon: Radio },
] as const;

const DRAG_PX = 6;
const SHEET_COLLAPSED_PX = 132;
const SHEET_EXPANDED_PX = 240;
const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus';

export function ReceiveScreen() {
  const {
    darkMode, setSubScreen, config, everyoneNearbySecondsLeft, showAllDevicesSheet,
    setShowAllDevicesSheet, missingPermissionsCount, formatCountdown, handleVisibilityChange,
    triggerBlePairing, openDeviceDetail, mapPins, visibilityIndex, pairingPrompt, transfers,
  } = useApp();

  const radarRef = useRef<HTMLDivElement | null>(null);
  const radarSize = useElementSize(radarRef);
  const sceneRef = useRef<HTMLDivElement | null>(null);
  const [dragPillIdx, setDragPillIdx] = useState<number | null>(null);
  const segTrackRef = useRef<HTMLDivElement | null>(null);
  const segDrag = useRef({ active: false, moved: false, id: -1, x: 0, base: 0, last: 0 });
  // While a pairing or transfer banner is up, the device list folds away so the radar keeps its room.
  const hasBanner = Boolean(pairingPrompt) || transfers.some((t) => t.state === 'pendingConsent' || t.state === 'active');
  const collapsedH = hasBanner ? 0 : SHEET_COLLAPSED_PX;
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
  const layout = radarLayoutFor(radarSize.width, radarSize.height);
  const visiblePins = mapPins.filter((pin) => pin.isVisible);
  useOrbitRate(sceneRef, radarActive, mapPins.length);

  return (
    <div className="kn-scroll-hidden flex flex-1 flex-col overflow-y-auto">
      <div className="relative z-20 shrink-0 space-y-2.5 px-4 pt-3">
        {missingPermissionsCount > 0 && (
          <button
            type="button"
            onClick={() => setSubScreen('permissions')}
            className={`animate-kn-fade-in flex min-h-(--kn-size-52px) w-full cursor-pointer items-center justify-between rounded-kn-2xl px-4 py-3 text-left transition-all duration-(--kn-duration-150) active:scale-(--kn-press-98) ${FOCUS_RING} kn-error-tonal`}
          >
            <span className="flex items-center gap-2.5 text-kn-sm font-kn-bold">
              <AlertTriangle className="h-5 w-5 shrink-0" aria-hidden="true" />
              <span>{pluralize(missingPermissionsCount, 'Android permission')} required</span>
            </span>
            <ChevronRight className="h-5 w-5 shrink-0" aria-hidden="true" />
          </button>
        )}

        <div className="rounded-kn-2xl p-1.5 kn-track-well">
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
                  className={`relative z-10 flex min-h-(--kn-size-48px) cursor-pointer items-center justify-center gap-2 rounded-kn-xl px-2.5 text-kn-sm font-kn-bold transition-colors duration-(--kn-duration-morph) ${FOCUS_RING} ${active ? 'kn-on-accent' : 'kn-text-secondary'}`}
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

      <div ref={radarRef} className="relative min-h-52 flex-1 overflow-hidden">
        <div className="pointer-events-none absolute inset-0">
          <WaveMapCanvas active={radarActive} darkMode={darkMode} />
        </div>
        <div
          aria-hidden="true"
          className={`pointer-events-none absolute inset-0 flex items-center justify-center transition-opacity duration-(--kn-duration-500) ${radarActive ? 'opacity-100' : 'opacity-0'}`}
        >
          <div className="h-(--kn-size-290px) w-(--kn-size-290px) rounded-full kn-brand-aura" />
        </div>

        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div ref={sceneRef} className="relative flex flex-col items-center" style={{ transform: `scale(${layout.scale})` }}>
            {/* Gradients cannot be transitioned, so the accent layer cross-fades over the glass one. */}
            <div className="relative flex h-16 w-16 items-center justify-center rounded-full kn-fog-surface">
              <span
                aria-hidden="true"
                className={`absolute inset-0 rounded-full kn-loading-bg transition-opacity duration-(--kn-duration-morph) ease-(--ease-kn-spring) ${radarActive ? 'opacity-100' : 'opacity-0'}`}
              />
              <Smartphone
                className={`relative h-7 w-7 transition-colors duration-(--kn-duration-morph) ${radarActive ? 'kn-on-accent' : 'kn-text-secondary'}`}
                aria-hidden="true"
              />
            </div>
            <span className="mt-2.5 rounded-kn-xl px-3.5 py-1.5 text-kn-xs font-kn-bold kn-fog-surface">
              Pixel 9 Pro
              <span
                aria-hidden={config.visibilityMode !== 'everyoneNearby' || undefined}
                className={`inline-block overflow-hidden whitespace-nowrap align-bottom tabular-nums kn-loading-text transition-[max-width,margin,opacity] duration-(--kn-duration-morph) ease-(--ease-kn-spring) ${config.visibilityMode === 'everyoneNearby' ? 'ml-1.5 max-w-16 opacity-100' : 'ml-0 max-w-0 opacity-0'}`}
              >
                {formatCountdown(everyoneNearbySecondsLeft)}
              </span>
            </span>

            <div className="absolute left-1/2 top-1/2 h-0 w-0">
              {mapPins.map((pin, idx) => {
                const orbit = orbitFor(pin.rssi, idx, mapPins.length, layout);
                const motion = {
                  animationDuration: `${orbit.duration}s`,
                  animationDelay: `-${orbit.phase}s`,
                } as const;
                return (
                  <div key={pin.id} className="animate-kn-orbit absolute left-0 top-0 h-0 w-0" style={motion}>
                    <div className="absolute left-0 top-0 h-0 w-0" style={{ translate: `${orbit.radius}px 0` }}>
                      <button
                        type="button"
                        onClick={() => (pin.isPaired && pin.rawDevice ? openDeviceDetail(pin.rawDevice) : triggerBlePairing(pin.displayName))}
                        style={motion}
                        tabIndex={pin.isVisible ? 0 : -1}
                        aria-hidden={!pin.isVisible || undefined}
                        className={`animate-kn-orbit-counter absolute left-0 top-0 z-10 flex min-h-(--kn-size-56px) min-w-(--kn-size-56px) -translate-x-1/2 -translate-y-1/2 cursor-pointer flex-col items-center rounded-kn-2xl text-center transition-[opacity,scale] duration-500 ease-(--ease-kn-spring) ${pin.isVisible ? 'pointer-events-auto scale-100 opacity-100' : 'pointer-events-none scale-75 opacity-0'} ${FOCUS_RING}`}
                      >
                        <div className="animate-kn-pin-enter flex flex-col items-center" style={{ animationDelay: `${idx * 55}ms` }}>
                          <div className="relative flex h-11 w-11 items-center justify-center rounded-full transition-all duration-(--kn-duration-press) ease-(--ease-kn-tactile) kn-fog-surface kn-fog-hover">
                            <Laptop className="h-5 w-5 kn-loading-text" aria-hidden="true" />
                          </div>
                          <div className="mt-2 rounded-kn-xl px-3.5 py-1.5 kn-fog-surface">
                            <div className="max-w-(--kn-size-136px) truncate text-kn-xs font-kn-bold leading-kn-tight">{pin.displayName}</div>
                            <div className="mt-0.5 max-w-(--kn-size-136px) truncate text-kn-xs leading-kn-tight tabular-nums kn-text-muted">
                              {joinMeta(pin.state, `${pin.rssi} dBm`)}
                            </div>
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
      </div>

      <section aria-label="Devices in range" className="relative z-20 mx-4 mb-2 shrink-0 rounded-kn-3xl px-4 pb-3 pt-2.5 kn-sheet">
        {visiblePins.length === 0 ? (
          <div className="flex flex-col items-center">
            <span aria-hidden="true" className="h-1.5 w-12 rounded-full kn-sheet-handle" />
            <div className="flex flex-col items-center gap-2 px-3 py-4 text-center">
              <span className="flex h-11 w-11 items-center justify-center rounded-kn-xl kn-loading-tonal">
                <Radio className="h-5 w-5" aria-hidden="true" />
              </span>
              <div className="text-kn-sm font-kn-bold kn-text-primary">No computers nearby</div>
              <div className="text-kn-xs kn-text-muted">
                Switch visibility to {VISIBILITY_LABELS.pairedOnly.full} or {VISIBILITY_LABELS.everyoneNearby.full}
              </div>
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
              className={`flex min-h-(--kn-size-44px) w-full cursor-pointer touch-none flex-col items-center justify-center rounded-kn-xl pb-1.5 pt-0.5 transition-opacity duration-(--kn-duration-150) active:opacity-75 ${FOCUS_RING}`}
            >
              <span aria-hidden="true" className="h-1.5 w-12 rounded-full kn-sheet-handle" />
              <div className="mt-2.5 flex w-full items-center justify-between">
                <span className="text-kn-sm font-kn-bold">{pluralize(visiblePins.length, 'computer')} in range</span>
                <span className="text-kn-sm font-kn-bold kn-loading-text">{showAllDevicesSheet ? 'Collapse' : 'All devices'}</span>
              </div>
            </button>

            <div
              style={{ maxHeight: `${sheetHeight}px` }}
              inert={sheetHeight === 0 || undefined}
              className={`overflow-y-auto kn-scroll-hidden ${dragSheetH === null ? 'transition-[max-height] duration-(--kn-duration-morph) ease-(--ease-kn-spring)' : ''}`}
            >
              {mapPins.map((pin) => (
                <div
                  key={pin.id}
                  inert={!pin.isVisible || undefined}
                  className={`grid transition-[grid-template-rows,opacity] duration-(--kn-duration-morph) ease-(--ease-kn-spring) ${pin.isVisible ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'}`}
                >
                  <div className="min-h-0 overflow-hidden">
                    <button
                      type="button"
                      onClick={() => (pin.isPaired && pin.rawDevice ? openDeviceDetail(pin.rawDevice) : triggerBlePairing(pin.displayName))}
                      className={`mb-1.5 flex min-h-(--kn-size-62px) w-full cursor-pointer items-center justify-between gap-3 rounded-kn-xl px-3 py-2.5 text-left transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-99) ${FOCUS_RING} kn-surface-hover`}
                    >
                      <div className="flex min-w-0 items-center gap-3">
                        <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-kn-xl kn-loading-tonal">
                          <Laptop className="h-5 w-5" aria-hidden="true" />
                        </div>
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="truncate text-kn-base font-kn-semibold">{pin.displayName}</span>
                            {!pin.isPaired && <span className="rounded-kn-lg px-2.5 py-0.5 text-kn-xs font-kn-bold kn-loading-tonal">Pair</span>}
                          </div>
                          <div className="truncate text-kn-xs tabular-nums kn-text-muted">{joinMeta(pin.state, `${pin.rssi} dBm`)}</div>
                        </div>
                      </div>
                      <ChevronRight className="h-5 w-5 shrink-0 kn-text-muted" aria-hidden="true" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
      </section>
    </div>
  );
}
