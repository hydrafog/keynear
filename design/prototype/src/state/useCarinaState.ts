import { useEffect, useState } from 'react';
import { DEMO_CALL_ALERT, DEMO_MIRRORED_NOTIFICATION, INITIAL_MEDIA_TRACKS } from '../data';
import type { CallAlert, MediaTrack, MirroredNotification } from '../types';

export interface CarinaStateParams {
  showSnackbar: (msg: string) => void;
  notificationMirrorEnabled: boolean;
}

export function useCarinaState({ showSnackbar, notificationMirrorEnabled }: CarinaStateParams) {
  const [mediaTracks, setMediaTracks] = useState<MediaTrack[]>(INITIAL_MEDIA_TRACKS);
  const [mediaIndex, setMediaIndex] = useState(0);
  const [mirroredNotification, setMirroredNotification] = useState<MirroredNotification | null>(null);
  const [callAlert, setCallAlert] = useState<CallAlert | null>(null);
  const currentMedia = mediaTracks[mediaIndex] ?? mediaTracks[0];

  useEffect(() => {
    if (!currentMedia?.isPlaying) return;
    const timer = setInterval(() => {
      setMediaTracks((prev) =>
        prev.map((track, idx) => {
          if (idx !== mediaIndex) return track;
          return track.positionSec + 1 >= track.durationSec
            ? { ...track, positionSec: 0, isPlaying: false }
            : { ...track, positionSec: track.positionSec + 1 };
        })
      );
    }, 1000);
    return () => clearInterval(timer);
  }, [currentMedia?.isPlaying, mediaIndex]);

  const toggleMediaPlay = () => {
    if (!currentMedia) return;
    const nextPlaying = !currentMedia.isPlaying;
    setMediaTracks((prev) =>
      prev.map((t, idx) => (idx === mediaIndex ? { ...t, isPlaying: nextPlaying } : { ...t, isPlaying: false }))
    );
    showSnackbar(
      `${nextPlaying ? 'Playing' : 'Paused'} ${currentMedia.title} on ${currentMedia.sourceDevice}.`
    );
  };

  const stepMediaTrack = (delta: 1 | -1) => {
    const nextIdx = (mediaIndex + delta + mediaTracks.length) % mediaTracks.length;
    const keepPlaying = Boolean(currentMedia?.isPlaying);
    setMediaIndex(nextIdx);
    setMediaTracks((prev) => prev.map((t, idx) => ({ ...t, isPlaying: idx === nextIdx ? keepPlaying : false })));
  };

  const nextMediaTrack = () => stepMediaTrack(1);
  const prevMediaTrack = () => stepMediaTrack(-1);

  const triggerMirroredNotification = () => {
    if (!notificationMirrorEnabled) {
      showSnackbar('Enable notification mirroring in Settings first.');
      return;
    }
    setMirroredNotification({
      ...DEMO_MIRRORED_NOTIFICATION,
      id: `mirror-${Date.now()}`,
      read: false,
      replySent: undefined,
    });
  };

  const replyMirroredNotification = (replyText: string) => {
    const trimmed = replyText.trim();
    if (!trimmed || !mirroredNotification) return;
    setMirroredNotification((prev) => (prev ? { ...prev, read: true, replySent: trimmed } : null));
    showSnackbar(`Sent reply to ${mirroredNotification.sender} via ${mirroredNotification.appName}.`);
  };

  const markMirroredNotificationRead = () => {
    if (!mirroredNotification) return;
    setMirroredNotification((prev) => (prev ? { ...prev, read: true } : null));
    showSnackbar(`Marked ${mirroredNotification.appName} notification as read.`);
  };

  const dismissMirroredNotification = () => {
    setMirroredNotification(null);
    showSnackbar('Dismissed mirrored notification.');
  };

  const triggerIncomingCall = () => {
    setCallAlert({ ...DEMO_CALL_ALERT, id: `call-${Date.now()}`, state: 'ringing' });
  };

  const answerCall = () => {
    if (!callAlert) return;
    setCallAlert((prev) => (prev ? { ...prev, state: 'active' } : null));
    showSnackbar(`Answered ${callAlert.callerName} via PipeWire ${callAlert.headsetProfile}.`);
  };

  const declineCall = () => {
    if (!callAlert) return;
    const caller = callAlert.callerName;
    setCallAlert(null);
    showSnackbar(`Declined call from ${caller}.`);
  };

  const toggleMuteCall = () => {
    if (!callAlert) return;
    const nextMuted = callAlert.state !== 'muted';
    setCallAlert((prev) => (prev ? { ...prev, state: nextMuted ? 'muted' : 'active' } : null));
    showSnackbar(nextMuted ? 'Call microphone muted.' : 'Call microphone unmuted.');
  };

  return {
    mediaTracks,
    mediaIndex,
    currentMedia,
    toggleMediaPlay,
    nextMediaTrack,
    prevMediaTrack,
    mirroredNotification,
    triggerMirroredNotification,
    replyMirroredNotification,
    markMirroredNotificationRead,
    dismissMirroredNotification,
    callAlert,
    triggerIncomingCall,
    answerCall,
    declineCall,
    toggleMuteCall,
  };
}
