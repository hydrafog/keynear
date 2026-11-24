import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type ReactNode,
} from 'react';
import { INITIAL_CONFIG, INITIAL_NEARBY_PEERS, INITIAL_PAIRED_DEVICES, INITIAL_PERMISSIONS, INITIAL_SEND_QUEUE, INITIAL_TRANSFERS } from '../data';
import { useCarinaState } from './useCarinaState';
import { pluralize } from '../lib/format';
import { applyAccent, DEFAULT_ACCENT } from '../lib/accent';
import type {
  ActiveTab,
  OtpPrompt,
  PairedDevice,
  PairingPrompt,
  SubScreen,
  TransferItem,
  TransferSession,
  VisibilityMode,
} from '../types';

export function useAppState() {
  const renderStampRef = useRef(0);
  renderStampRef.current += 1;
  const renderStamp = renderStampRef.current;

  const [darkMode, setDarkMode] = useState(false);
  const [accentColor, setAccentColor] = useState(DEFAULT_ACCENT);
  const [activeTab, setActiveTab] = useState<ActiveTab>('receive');
  const [subScreen, setSubScreen] = useState<SubScreen>(null);

  const [shadeOpen, setShadeOpen] = useState(false);
  const [shareSheetOpen, setShareSheetOpen] = useState(false);
  const [lockScreenOpen, setLockScreenOpen] = useState(false);
  const [isDeviceLocked, setIsDeviceLocked] = useState(false);
  const [pendingUnlockAction, setPendingUnlockAction] = useState<(() => void) | null>(null);

  const [config, setConfig] = useState(INITIAL_CONFIG);
  const [permissions, setPermissions] = useState(INITIAL_PERMISSIONS);
  const [pairedDevices, setPairedDevices] = useState(INITIAL_PAIRED_DEVICES);
  const [nearbyPeers, setNearbyPeers] = useState(INITIAL_NEARBY_PEERS);
  const [everyoneNearbySecondsLeft, setEveryoneNearbySecondsLeft] = useState(600);

  const [sendQueue, setSendQueue] = useState(INITIAL_SEND_QUEUE);
  const [showTextComposer, setShowTextComposer] = useState(false);
  const [customTextSnippet, setCustomTextSnippet] = useState('');

  const [selectedDeviceId, setSelectedDeviceId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState('');
  const [confirmUnpairId, setConfirmUnpairId] = useState<string | null>(null);
  const [showMimeEditor, setShowMimeEditor] = useState(false);
  const [newMimeInput, setNewMimeInput] = useState('');
  const [snackbarMessage, setSnackbarMessage] = useState<string | null>(null);
  const [showAllDevicesSheet, setShowAllDevicesSheet] = useState(false);

  const [transfers, setTransfers] = useState(INITIAL_TRANSFERS);
  const [dismissedTransferIds, setDismissedTransferIds] = useState<Set<string>>(() => new Set());
  const [expandedNotifIds, setExpandedNotifIds] = useState<Record<string, boolean>>({ 'tx-initial-1': true });
  const [pairingPrompt, setPairingPrompt] = useState<PairingPrompt | null>(null);
  const [otpPrompt, setOtpPrompt] = useState<OtpPrompt | null>(null);

  useEffect(() => {
    if (config.visibilityMode !== 'everyoneNearby') return;
    const timer = setInterval(() => {
      setEveryoneNearbySecondsLeft((prev) => {
        if (prev <= 1) {
          setConfig((c) => ({ ...c, visibilityMode: 'pairedOnly' }));
          showSnackbar('Everyone nearby expired. Reverted to Paired only.');
          return 600;
        }
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(timer);
  }, [config.visibilityMode]);

  useEffect(() => {
    if (!pairingPrompt) return;
    const timer = setInterval(() => {
      setPairingPrompt((prev) => {
        if (!prev) return null;
        if (prev.secondsLeft <= 1) {
          showSnackbar('Pairing verification timed out after 120 seconds.');
          return null;
        }
        return { ...prev, secondsLeft: prev.secondsLeft - 1 };
      });
    }, 1000);
    return () => clearInterval(timer);
  }, [pairingPrompt]);

  useEffect(() => {
    const hasActive = transfers.some((t) => t.state === 'active');
    if (!hasActive) return;

    const interval = setInterval(() => {
      setTransfers((prev) =>
        prev.map((t) => {
          if (t.state !== 'active') return t;
          const step = Math.max(t.totalBytes * 0.14, 2_400_000);
          const nextBytes = Math.min(t.totalBytes, t.transferredBytes + step);
          const isDone = nextBytes >= t.totalBytes;
          if (isDone) {
            showSnackbar(
              t.direction === 'receive'
                ? `Saved ${pluralize(t.items.length, 'item')} to ${config.incomingDirectory}`
                : `Sent ${pluralize(t.items.length, 'item')} to ${t.peerName}`
            );
          }
          return {
            ...t,
            transferredBytes: nextBytes,
            state: isDone ? 'completed' : 'active',
          };
        })
      );
    }, 650);

    return () => clearInterval(interval);
  }, [transfers, config.incomingDirectory]);

  const showSnackbar = (msg: string) => {
    setSnackbarMessage(msg);
    setTimeout(() => {
      setSnackbarMessage((current) => (current === msg ? null : current));
    }, 3200);
  };

  const carina = useCarinaState({
    showSnackbar,
    notificationMirrorEnabled: config.notificationMirrorEnabled,
  });

  const missingPermissionsCount = useMemo(
    () =>
      Object.entries(permissions).filter(
        ([key, granted]) => key !== 'companionAssociation' && !granted
      ).length,
    [permissions]
  );

  const activeTransfer = useMemo(
    () => transfers.find((t) => t.state === 'active') || null,
    [transfers]
  );

  const pendingShadeCount = useMemo(() => {
    let count = 0;
    if (pairingPrompt) count += 1;
    if (otpPrompt) count += 1;
    if (carina.mirroredNotification) count += 1;
    if (carina.callAlert) count += 1;
    count += transfers.filter((t) => t.state === 'pendingConsent' || t.state === 'active').length;
    return count;
  }, [
    pairingPrompt,
    otpPrompt,
    carina.mirroredNotification,
    carina.callAlert,
    transfers,
  ]);

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  const formatCountdown = (sec: number) => {
    const m = Math.floor(sec / 60);
    const s = sec % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
  };

  const handleVisibilityChange = (nextMode: VisibilityMode) => {
    if (nextMode === 'everyoneNearby' && isDeviceLocked) {
      setPendingUnlockAction(() => () => {
        setConfig((c) => ({ ...c, visibilityMode: 'everyoneNearby' }));
        setEveryoneNearbySecondsLeft(600);
        showSnackbar('Visible to everyone nearby for 10 minutes.');
      });
      setShadeOpen(false);
      setLockScreenOpen(true);
      return;
    }
    setConfig((c) => ({ ...c, visibilityMode: nextMode }));
    if (nextMode === 'everyoneNearby') {
      setEveryoneNearbySecondsLeft(600);
      showSnackbar('Visible to everyone nearby for 10 minutes.');
    } else if (nextMode === 'pairedOnly') {
      showSnackbar('Listening for paired computers.');
    } else {
      showSnackbar('Visibility set to hidden.');
    }
  };

  const handleCycleQuickSettingsTile = () => {
    const order: VisibilityMode[] = ['hidden', 'pairedOnly', 'everyoneNearby'];
    const nextIdx = (order.indexOf(config.visibilityMode) + 1) % order.length;
    handleVisibilityChange(order[nextIdx]);
  };

  const handleUnlockDevice = () => {
    setIsDeviceLocked(false);
    setLockScreenOpen(false);
    if (pendingUnlockAction) {
      pendingUnlockAction();
      setPendingUnlockAction(null);
    }
  };

  const triggerIncomingDrop = (fromPaired = false) => {
    const peer = fromPaired
      ? pairedDevices[0]
      : { deviceId: 'unpaired-fedora', displayName: 'fedora-studio', autoAcceptFiles: false };

    const items = fromPaired
      ? [
          { name: 'release-notes-v2.4.md', sizeBytes: 184_000, isDirectory: false },
          { name: 'screenshots/', sizeBytes: 24_500_000, isDirectory: true },
        ]
      : [
          { name: 'kernel-build-logs.tar.zst', sizeBytes: 42_600_000, isDirectory: false },
          { name: 'wireguard-configs/', sizeBytes: 18_200_000, isDirectory: true },
        ];

    const totalBytes = items.reduce((acc, i) => acc + i.sizeBytes, 0);
    const shouldAutoAccept = fromPaired && peer?.autoAcceptFiles && config.autoAcceptFromPaired;

    const newTransfer: TransferSession = {
      transferId: `tx-${Date.now()}`,
      peerDeviceId: peer?.deviceId || 'unpaired-fedora',
      peerName: peer?.displayName || 'fedora-studio',
      direction: 'receive',
      items,
      totalBytes,
      transferredBytes: shouldAutoAccept ? totalBytes * 0.16 : 0,
      state: shouldAutoAccept ? 'active' : 'pendingConsent',
    };

    setTransfers((prev) => [newTransfer, ...prev.filter((t) => t.state !== 'completed')]);
    setExpandedNotifIds((prev) => ({ ...prev, [newTransfer.transferId]: true }));
    setShadeOpen(true);
  };

  const handleTransferConsent = (transferId: string, accept: boolean) => {
    setTransfers((prev) =>
      prev.map((t) => {
        if (t.transferId !== transferId) return t;
        if (!accept) return { ...t, state: 'cancelled' };
        return { ...t, state: 'active', transferredBytes: t.totalBytes * 0.12 };
      })
    );
    if (!accept) {
      showSnackbar('Incoming transfer declined.');
    }
  };

  const handleCancelTransfer = (transferId: string) => {
    setTransfers((prev) =>
      prev.map((t) => (t.transferId === transferId ? { ...t, state: 'cancelled' } : t))
    );
    showSnackbar('Transfer cancelled.');
  };

  const dismissTransferNotification = (transferId: string) => {
    setTransfers((prev) => prev.filter((t) => t.transferId !== transferId));
  };

  const dismissTransfers = useCallback((ids: string[]) => {
    setDismissedTransferIds((prev) => new Set([...prev, ...ids]));
  }, []);

  const triggerBlePairing = (targetPeerName = 'fedora-studio') => {
    const code = `${Math.floor(100 + Math.random() * 900)} ${Math.floor(100 + Math.random() * 900)}`;
    setPairingPrompt({
      peerName: targetPeerName,
      verificationCode: code,
      pubkeyFingerprint: '7c19:e840:3b21:9a5f:0d84:6e12:4c77:81a0',
      bleAddress: '4C:82:19:F0:3A:7D',
      secondsLeft: 120,
    });
    setShadeOpen(true);
  };

  const handlePairingDecision = (approve: boolean) => {
    if (!pairingPrompt) return;
    if (approve) {
      const exists = pairedDevices.some((d) => d.displayName === pairingPrompt.peerName);
      if (!exists) {
        const added: PairedDevice = {
          deviceId: `dev-${Math.random().toString(16).slice(2, 10)}`,
          displayName: pairingPrompt.peerName,
          staticX25519Pubkey: pairingPrompt.pubkeyFingerprint,
          lastBleAddress: pairingPrompt.bleAddress,
          lastLanAddress: '192.168.1.73:4433',
          autoAcceptFiles: false,
          pairedAt: 'Nov 12, 2026',
          isOnline: true,
          connectionLabel: 'Connected via Wi-Fi',
          rssi: -56,
        };
        setPairedDevices((prev) => [added, ...prev]);
        setNearbyPeers((prev) => prev.filter((p) => p.displayName !== pairingPrompt.peerName));
      }
      showSnackbar(`Paired ${pairingPrompt.peerName} and saved keys to Android Keystore.`);
    } else {
      showSnackbar('Pairing request rejected.');
    }
    setPairingPrompt(null);
  };

  const triggerSmsOtpConsent = () => {
    if (!config.otpPassthroughEnabled) {
      showSnackbar('Enable 2FA code pass-through in Settings first.');
      return;
    }
    setOtpPrompt({
      sender: '+40 721 ••• 890',
      code: '739104',
      targetDesktop: pairedDevices[0]?.displayName || 'arch-workstation',
    });
    setShadeOpen(true);
  };

  const handlePushOtp = () => {
    if (!otpPrompt) return;
    showSnackbar(`Copied ${otpPrompt.code} to ${otpPrompt.targetDesktop} clipboard.`);
    setOtpPrompt(null);
  };

  const handleSendClipboardFromTile = () => {
    if (!config.clipboardSyncEnabled) {
      showSnackbar('Clipboard sync is disabled in Settings.');
      return;
    }
    const target = pairedDevices.find((d) => d.isOnline) || pairedDevices[0];
    setShadeOpen(false);
    showSnackbar(`Sent clipboard contents to ${target?.displayName || 'desktop'}.`);
  };

  const startOutgoingTransfer = (
    targetPeer: Pick<PairedDevice, 'deviceId' | 'displayName'>,
    itemsToSend: TransferItem[]
  ) => {
    if (!itemsToSend || itemsToSend.length === 0) {
      showSnackbar('Select at least one file, folder, or note first.');
      return;
    }
    const totalBytes = itemsToSend.reduce((acc, i) => acc + i.sizeBytes, 0);
    const outgoing: TransferSession = {
      transferId: `tx-out-${Date.now()}`,
      peerDeviceId: targetPeer.deviceId,
      peerName: targetPeer.displayName,
      direction: 'send',
      items: itemsToSend,
      totalBytes,
      transferredBytes: totalBytes * 0.18,
      state: 'active',
    };
    setTransfers((prev) => [outgoing, ...prev.filter((t) => t.state !== 'completed')]);
    setExpandedNotifIds((prev) => ({ ...prev, [outgoing.transferId]: true }));
    setShareSheetOpen(false);
    setShadeOpen(true);
  };

  const selectedDevice = useMemo(
    () => pairedDevices.find((d) => d.deviceId === selectedDeviceId) || null,
    [pairedDevices, selectedDeviceId]
  );

  const openDeviceDetail = (device: PairedDevice) => {
    setSelectedDeviceId(device.deviceId);
    setEditingName(device.displayName);
    setConfirmUnpairId(null);
    setSubScreen('deviceDetail');
  };

  const saveDeviceName = (e: FormEvent) => {
    e.preventDefault();
    if (!selectedDevice || !editingName.trim()) return;
    setPairedDevices((prev) =>
      prev.map((d) => (d.deviceId === selectedDevice.deviceId ? { ...d, displayName: editingName.trim() } : d))
    );
    showSnackbar('Device name saved.');
  };

  const togglePerDeviceAutoAccept = (deviceId: string) => {
    setPairedDevices((prev) =>
      prev.map((d) => (d.deviceId === deviceId ? { ...d, autoAcceptFiles: !d.autoAcceptFiles } : d))
    );
  };

  const unpairDevice = (deviceId: string) => {
    const dev = pairedDevices.find((d) => d.deviceId === deviceId);
    setPairedDevices((prev) => prev.filter((d) => d.deviceId !== deviceId));
    setConfirmUnpairId(null);
    setSubScreen(null);
    setSelectedDeviceId(null);
    showSnackbar(`Removed ${dev?.displayName || 'device'}.`);
  };

  const addIgnoredMime = (e: FormEvent) => {
    e.preventDefault();
    const trimmed = newMimeInput.trim();
    if (!trimmed || config.ignoredMimeTypes.includes(trimmed)) return;
    setConfig((c) => ({ ...c, ignoredMimeTypes: [...c.ignoredMimeTypes, trimmed] }));
    setNewMimeInput('');
  };

  const removeIgnoredMime = (mime: string) => {
    setConfig((c) => ({
      ...c,
      ignoredMimeTypes: c.ignoredMimeTypes.filter((m) => m !== mime),
    }));
  };

  const mapPins = useMemo(() => {
    const pairedPins = pairedDevices.map((dev) => ({
      id: dev.deviceId,
      displayName: dev.displayName,
      state: dev.isOnline ? 'Wi-Fi' : 'Standby',
      isPaired: true,
      isVisible: true,
      isOnline: dev.isOnline,
      autoAcceptFiles: dev.autoAcceptFiles,
      rssi: dev.rssi,
      rawDevice: dev,
    }));

    // Unpaired peers stay in the list while hidden so the radar can fade them out
    // instead of unmounting them; `isVisible` tells consumers whether they count.
    const nearbyPins = nearbyPeers.map((peer) => ({
      id: peer.presenceToken,
      displayName: peer.displayName,
      state: 'Tap to pair',
      isPaired: false,
      isVisible: config.visibilityMode !== 'hidden',
      isOnline: true,
      autoAcceptFiles: false,
      rssi: peer.rssi,
      rawDevice: null,
    }));

    return [...pairedPins, ...nearbyPins];
  }, [pairedDevices, nearbyPeers, config.visibilityMode]);

  const activePct = activeTransfer
    ? Math.min(100, Math.round((activeTransfer.transferredBytes / activeTransfer.totalBytes) * 100))
    : 0;

  const visibilityIndex = ['hidden', 'pairedOnly', 'everyoneNearby'].indexOf(config.visibilityMode);

  useLayoutEffect(() => {
    document.documentElement.dataset.knTheme = darkMode ? 'dark' : 'light';
  }, [darkMode]);

  useEffect(() => {
    applyAccent(accentColor, darkMode);
  }, [accentColor, darkMode]);

  return {
    darkMode, setDarkMode, accentColor, setAccentColor, activeTab, setActiveTab, subScreen, setSubScreen, shadeOpen, setShadeOpen, shareSheetOpen, setShareSheetOpen, lockScreenOpen, setLockScreenOpen, isDeviceLocked, setIsDeviceLocked, pendingUnlockAction, setPendingUnlockAction, config, setConfig, permissions, setPermissions, pairedDevices, setPairedDevices, nearbyPeers, setNearbyPeers, everyoneNearbySecondsLeft, setEveryoneNearbySecondsLeft, sendQueue, setSendQueue, showTextComposer, setShowTextComposer, customTextSnippet, setCustomTextSnippet, selectedDeviceId, setSelectedDeviceId, editingName, setEditingName, confirmUnpairId, setConfirmUnpairId, showMimeEditor, setShowMimeEditor, newMimeInput, setNewMimeInput, snackbarMessage, setSnackbarMessage, showAllDevicesSheet, setShowAllDevicesSheet, transfers, setTransfers, dismissedTransferIds, dismissTransfers, expandedNotifIds, setExpandedNotifIds, pairingPrompt, setPairingPrompt, otpPrompt, setOtpPrompt, showSnackbar, missingPermissionsCount, activeTransfer, pendingShadeCount, formatBytes, formatCountdown, handleVisibilityChange, handleCycleQuickSettingsTile, handleUnlockDevice, triggerIncomingDrop, handleTransferConsent, handleCancelTransfer, dismissTransferNotification, triggerBlePairing, handlePairingDecision, triggerSmsOtpConsent, handlePushOtp, handleSendClipboardFromTile, startOutgoingTransfer, selectedDevice, openDeviceDetail, saveDeviceName, togglePerDeviceAutoAccept, unpairDevice, addIgnoredMime, removeIgnoredMime, mapPins, activePct, visibilityIndex, renderStamp, ...carina,
  };
}

export type AppState = ReturnType<typeof useAppState>;

const AppContext = createContext<AppState | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const state = useAppState();
  return <AppContext.Provider value={state}>{children}</AppContext.Provider>;
}

export function useApp(): AppState {
  const state = useContext(AppContext);
  if (state === null) throw new Error('useApp must be used inside AppProvider');
  return state;
}
