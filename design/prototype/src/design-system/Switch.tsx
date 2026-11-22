import { useRef, useState } from 'react';
import type { MouseEvent, PointerEvent } from 'react';

type SwitchProps = { checked: boolean; onChange: (next: boolean) => void; label: string };
type DragSession = { pointerId: number; startX: number; baseOffset: number; moved: boolean; offset: number };

const THUMB_TRAVEL_PX = 24;
const SNAP_MIDPOINT_PX = 12;
const DRAG_THRESHOLD_PX = 3;

function releaseCapture(e: PointerEvent<HTMLButtonElement>) {
  if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
}

function useSwitchDrag(checked: boolean, onChange: (next: boolean) => void) {
  const dragRef = useRef<DragSession | null>(null);
  const suppressClickRef = useRef(false);
  const [dragOffset, setDragOffset] = useState<number | null>(null);

  const onPointerDown = (e: PointerEvent<HTMLButtonElement>) => {
    if (e.pointerType === 'mouse' && e.button !== 0) return;
    e.stopPropagation();
    e.currentTarget.setPointerCapture(e.pointerId);
    const baseOffset = checked ? THUMB_TRAVEL_PX : 0;
    dragRef.current = { pointerId: e.pointerId, startX: e.clientX, baseOffset, moved: false, offset: baseOffset };
  };

  const onPointerMove = (e: PointerEvent<HTMLButtonElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== e.pointerId) return;
    const delta = e.clientX - drag.startX;
    if (!drag.moved && Math.abs(delta) >= DRAG_THRESHOLD_PX) drag.moved = true;
    if (!drag.moved) return;
    drag.offset = Math.max(0, Math.min(THUMB_TRAVEL_PX, drag.baseOffset + delta));
    setDragOffset(drag.offset);
  };

  const finishDrag = (e: PointerEvent<HTMLButtonElement>, commit: boolean) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== e.pointerId) return;
    releaseCapture(e);
    dragRef.current = null;
    setDragOffset(null);
    if (!commit || !drag.moved) return;
    suppressClickRef.current = true;
    window.setTimeout(() => { suppressClickRef.current = false; }, 0);
    const nextChecked = drag.offset >= SNAP_MIDPOINT_PX;
    if (nextChecked !== checked) onChange(nextChecked);
  };

  const onClick = (e: MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    if (suppressClickRef.current) { suppressClickRef.current = false; return; }
    onChange(!checked);
  };

  const onPointerUp = (e: PointerEvent<HTMLButtonElement>) => finishDrag(e, true);
  const onPointerCancel = (e: PointerEvent<HTMLButtonElement>) => finishDrag(e, false);
  return { dragOffset, handlers: { onPointerDown, onPointerMove, onPointerUp, onPointerCancel, onClick } };
}

export function Switch({ checked, onChange, label }: SwitchProps) {
  const { dragOffset, handlers } = useSwitchDrag(checked, onChange);
  const isDragging = dragOffset !== null;
  const visualChecked = isDragging ? dragOffset >= SNAP_MIDPOINT_PX : checked;

  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      {...handlers}
      className="flex min-h-(--kn-size-52px) min-w-(--kn-size-62px) shrink-0 cursor-pointer touch-pan-y select-none items-center justify-end rounded-full focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500"
    >
      <span
        className={`relative inline-flex h-(--kn-size-34px) w-(--kn-size-58px) items-center rounded-full p-(--kn-size-4px) transition-[background-color,background-image,box-shadow] duration-(--kn-duration-switch) ease-(--ease-kn-spring) ${
          visualChecked ? 'kn-switch-track-on' : 'kn-switch-track-off'
        }`}
      >
        <span
          style={isDragging ? { transform: `translateX(${dragOffset}px)` } : undefined}
          className={`inline-block h-(--kn-size-26px) w-(--kn-size-26px) rounded-full kn-switch-thumb ${
            isDragging
              ? ''
              : `transition-[transform,box-shadow] duration-(--kn-duration-switch) ease-(--ease-kn-spring) ${
                  checked ? 'translate-x-(--kn-size-24px)' : 'translate-x-0'
                }`
          }`}
        />
      </span>
    </button>
  );
}
