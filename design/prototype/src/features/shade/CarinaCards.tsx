import { useState, type FormEvent } from 'react';
import {
  Headphones, MessageSquare, Mic, MicOff, Pause,
  PhoneCall, PhoneOff, Play, Send, SkipBack, SkipForward,
} from 'lucide-react';
import { useApp } from '../../state/AppProvider';
import { NotificationCard, NotificationHeader } from '../../design-system/Notifications';

const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-blue-500';
const MEDIA_ICON_BTN = `flex h-11 w-11 min-h-(--kn-size-44px) min-w-(--kn-size-44px) cursor-pointer items-center justify-center rounded-full text-white/90 transition-colors duration-(--kn-duration-150) hover:bg-white/15 active:scale-(--kn-press-95) ${FOCUS_RING}`;
const SYS_PILL_BTN = `min-h-(--kn-size-44px) cursor-pointer rounded-full px-3.5 text-kn-xs font-kn-semibold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) kn-sys-notif-action ${FOCUS_RING}`;
const CALL_PILL_BTN = `inline-flex min-h-(--kn-size-44px) flex-1 cursor-pointer items-center justify-center gap-1.5 rounded-full px-3 text-kn-xs font-kn-bold transition-transform duration-(--kn-duration-150) active:scale-(--kn-press-95) ${FOCUS_RING}`;

export function MediaCarouselCard() {
  const { currentMedia, toggleMediaPlay, nextMediaTrack, prevMediaTrack, formatCountdown } = useApp();
  if (!currentMedia) return null;
  const pct = currentMedia.durationSec > 0
    ? Math.min(100, Math.round((currentMedia.positionSec / currentMedia.durationSec) * 100))
    : 0;

  return (
    <section aria-label="Now playing media controls" className="rounded-3xl p-4 kn-sys-media">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 text-kn-xs text-white/80">
          <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-white/15">
            <Headphones className="h-3 w-3 text-white" aria-hidden="true" />
          </span>
          <span className="font-kn-semibold">Keynear</span>
        </div>
        <span className="truncate rounded-full bg-white/15 px-2.5 py-1 text-kn-xs font-kn-semibold text-white/90">
          {currentMedia.sourceDevice} · {currentMedia.playerName}
        </span>
      </div>

      <div className="mt-3 min-w-0">
        <div className="truncate text-kn-base font-kn-bold text-white">{currentMedia.title}</div>
        <div className="truncate text-kn-xs text-white/75">{currentMedia.artist} · {currentMedia.album}</div>
      </div>

      <div className="mt-3">
        <div
          role="progressbar"
          aria-valuenow={pct}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label="Track playback progress"
          className="h-1 w-full overflow-hidden rounded-full bg-white/25"
        >
          <div className="h-full rounded-full bg-white transition-[width] duration-(--kn-duration-200) ease-out" style={{ width: `${pct}%` }} />
        </div>
        <div className="mt-1.5 flex justify-between text-kn-xs tabular-nums text-white/75">
          <span>{formatCountdown(currentMedia.positionSec)}</span>
          <span>{formatCountdown(currentMedia.durationSec)}</span>
        </div>
      </div>

      <div className="mt-2.5 flex items-center justify-center gap-4">
        <button type="button" onClick={prevMediaTrack} aria-label="Previous track" className={MEDIA_ICON_BTN}>
          <SkipBack className="h-5 w-5" aria-hidden="true" />
        </button>
        <button
          type="button"
          onClick={toggleMediaPlay}
          aria-label={currentMedia.isPlaying ? 'Pause playback' : 'Play track'}
          className={`flex h-12 w-12 min-h-(--kn-size-48px) min-w-(--kn-size-48px) cursor-pointer items-center justify-center rounded-2xl bg-white text-[#0B1931] transition-transform duration-(--kn-duration-150) active:scale-(--kn-press-95) ${FOCUS_RING}`}
        >
          {currentMedia.isPlaying ? <Pause className="h-5 w-5" aria-hidden="true" /> : <Play className="h-5 w-5" aria-hidden="true" />}
        </button>
        <button type="button" onClick={nextMediaTrack} aria-label="Next track" className={MEDIA_ICON_BTN}>
          <SkipForward className="h-5 w-5" aria-hidden="true" />
        </button>
      </div>
    </section>
  );
}

export function MirroredNotificationCard() {
  const {
    mirroredNotification, replyMirroredNotification,
    markMirroredNotificationRead, dismissMirroredNotification,
  } = useApp();
  const [replyOpen, setReplyOpen] = useState(false);
  const [replyDraft, setReplyDraft] = useState('');
  if (!mirroredNotification) return null;

  const handleReplySubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!replyDraft.trim()) return;
    replyMirroredNotification(replyDraft);
    setReplyDraft('');
    setReplyOpen(false);
  };

  return (
    <NotificationCard>
      <div className="flex items-center justify-between gap-2">
        <NotificationHeader
          icon={<MessageSquare className="h-3 w-3" aria-hidden="true" />}
          label={`${mirroredNotification.appName} · ${mirroredNotification.timestamp}`}
        />
        {mirroredNotification.read && (
          <span className="shrink-0 text-kn-xs font-kn-semibold kn-loading-text">Read</span>
        )}
      </div>

      <div className="mt-2.5 flex items-start gap-3">
        <span aria-hidden="true" className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full text-kn-sm font-kn-bold kn-loading-tonal">
          {mirroredNotification.sender.slice(0, 1).toUpperCase()}
        </span>
        <div className="min-w-0 flex-1">
          <div className="truncate text-kn-sm font-kn-bold">{mirroredNotification.sender}</div>
          <p className="mt-0.5 text-kn-sm leading-kn-tight kn-text-secondary">{mirroredNotification.body}</p>
          {mirroredNotification.replySent && (
            <p className="mt-1 text-kn-xs kn-text-muted">You: {mirroredNotification.replySent}</p>
          )}
        </div>
      </div>

      {replyOpen && (
        <form onSubmit={handleReplySubmit} className="mt-3 flex items-center gap-2">
          <label htmlFor="mirrored-reply-input" className="sr-only">Reply to {mirroredNotification.sender}</label>
          <input
            id="mirrored-reply-input"
            type="text"
            autoFocus
            value={replyDraft}
            onChange={(e) => setReplyDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                e.nativeEvent.stopPropagation();
                setReplyOpen(false);
              }
            }}
            placeholder="Reply"
            className={`min-h-(--kn-size-44px) min-w-0 flex-1 rounded-full px-4 text-kn-base kn-sys-notif-action ${FOCUS_RING}`}
          />
          <button
            type="submit"
            disabled={!replyDraft.trim()}
            aria-label="Send reply"
            className={`flex h-11 w-11 shrink-0 cursor-pointer items-center justify-center rounded-full transition-opacity duration-(--kn-duration-150) active:scale-(--kn-press-95) disabled:opacity-45 kn-sys-qs-on ${FOCUS_RING}`}
          >
            <Send className="h-4 w-4" aria-hidden="true" />
          </button>
        </form>
      )}

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <button type="button" aria-expanded={replyOpen} onClick={() => setReplyOpen((open) => !open)} className={`kn-loading-text ${SYS_PILL_BTN}`}>
          Reply
        </button>
        <button type="button" disabled={mirroredNotification.read} onClick={markMirroredNotificationRead} className={`disabled:opacity-45 ${SYS_PILL_BTN}`}>
          Mark as read
        </button>
        <button type="button" onClick={dismissMirroredNotification} className={SYS_PILL_BTN}>
          Dismiss
        </button>
      </div>
    </NotificationCard>
  );
}

export function CallAlertCard() {
  const { callAlert, answerCall, declineCall, toggleMuteCall } = useApp();
  if (!callAlert) return null;
  const isRinging = callAlert.state === 'ringing';
  const isMuted = callAlert.state === 'muted';

  return (
    <NotificationCard>
      <NotificationHeader
        icon={<PhoneCall className="h-3 w-3" aria-hidden="true" />}
        label={isRinging ? 'Incoming call · Keynear' : 'Ongoing call · Keynear'}
      />

      <div className="mt-2.5 flex items-center gap-3">
        <span aria-hidden="true" className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full text-kn-base font-kn-bold kn-loading-tonal">
          {callAlert.callerName.slice(0, 1).toUpperCase()}
        </span>
        <div className="min-w-0 flex-1">
          <div className="truncate text-kn-base font-kn-bold">{callAlert.callerName}</div>
          <div className="truncate text-kn-xs tabular-nums kn-text-muted">{callAlert.phoneNumber}</div>
          <div className="mt-0.5 truncate text-kn-xs kn-text-muted">Headset handoff ({callAlert.headsetProfile})</div>
        </div>
      </div>

      <div className="mt-3 flex gap-2">
        {isRinging && (
          <button type="button" onClick={answerCall} className={`kn-sys-call-accept ${CALL_PILL_BTN}`}>
            <PhoneCall className="h-4 w-4" aria-hidden="true" />
            <span>Answer</span>
          </button>
        )}
        <button type="button" onClick={declineCall} className={`kn-sys-call-decline ${CALL_PILL_BTN}`}>
          <PhoneOff className="h-4 w-4" aria-hidden="true" />
          <span>Decline</span>
        </button>
        <button type="button" onClick={toggleMuteCall} aria-pressed={isMuted} className={`kn-sys-notif-action font-kn-semibold ${CALL_PILL_BTN}`}>
          {isMuted ? <MicOff className="h-4 w-4" aria-hidden="true" /> : <Mic className="h-4 w-4" aria-hidden="true" />}
          <span>{isMuted ? 'Unmute' : 'Mute'}</span>
        </button>
      </div>
    </NotificationCard>
  );
}
