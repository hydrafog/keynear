import type {
  CallAlert,
  DaemonConfig,
  MediaTrack,
  MirroredNotification,
  NearbyPeer,
  PairedDevice,
  PermissionsState,
  SendQueueItem,
  TransferSession,
} from './types';

export const INITIAL_CONFIG: DaemonConfig = {
  visibilityMode: 'pairedOnly',
  incomingDirectory: 'Downloads/Keynear',
  autoAcceptFromPaired: true,
  clipboardSyncEnabled: true,
  clipboardImagesEnabled: true,
  otpPassthroughEnabled: true,
  audioHandoffEnabled: true,
  ignoredMimeTypes: ['x-kde-passwordManagerHint', 'application/x-password'],
  notifyAutoAcceptComplete: true,
  notificationMirrorEnabled: true,
};

export const INITIAL_PERMISSIONS: PermissionsState = {
  nearbyDevices: true,
  localNetwork: true,
  notifications: true,
  companionAssociation: true,
  batteryUnrestricted: true,
};

export const INITIAL_PAIRED_DEVICES: PairedDevice[] = [
  {
    deviceId: 'dev-9f8e7d6c-4a2b-11ef',
    displayName: 'arch-workstation',
    staticX25519Pubkey: '8f3a:91c4:0e72:b5d1:66a9:4c18:f023:77e4',
    lastBleAddress: '5A:91:E4:20:8B:1C',
    lastLanAddress: '[fe80::1a2b:3c4d:5e6f]:4433',
    autoAcceptFiles: true,
    pairedAt: 'Oct 28, 2026',
    isOnline: true,
    connectionLabel: 'Connected via Wi-Fi',
    rssi: -52,
  },
  {
    deviceId: 'dev-3c1b0a9e-8d7f-11ef',
    displayName: 'thinkpad-x1',
    staticX25519Pubkey: '2b7e:44a1:c908:11f3:84d2:5e60:9a17:33b9',
    lastBleAddress: '6E:12:9C:77:4F:03',
    lastLanAddress: '192.168.1.48:4433',
    autoAcceptFiles: false,
    pairedAt: 'Nov 2, 2026',
    isOnline: false,
    connectionLabel: 'Bluetooth standby',
    rssi: -78,
  },
];

export const INITIAL_NEARBY_PEERS: NearbyPeer[] = [
  {
    presenceToken: '9A4F12C8E103',
    displayName: 'fedora-studio',
    rssi: -58,
  },
];

export const INITIAL_SEND_QUEUE: SendQueueItem[] = [
  {
    id: 'item-1',
    name: 'PXL_20261112_214012.jpg',
    sizeBytes: 4_820_000,
    isDirectory: false,
  },
  {
    id: 'item-2',
    name: 'field-recordings/',
    sizeBytes: 38_400_000,
    isDirectory: true,
  },
];

export const INITIAL_TRANSFERS: TransferSession[] = [
  {
    transferId: 'tx-initial-1',
    peerDeviceId: 'unpaired-fedora',
    peerName: 'fedora-studio',
    direction: 'receive',
    items: [
      {
        name: 'kernel-build-logs.tar.zst',
        sizeBytes: 42_600_000,
        isDirectory: false,
      },
      {
        name: 'wireguard-configs/',
        sizeBytes: 18_200_000,
        isDirectory: true,
      },
    ],
    totalBytes: 60_800_000,
    transferredBytes: 0,
    state: 'pendingConsent',
  },
];

export const INITIAL_MEDIA_TRACKS: MediaTrack[] = [
  {
    id: 'track-1',
    title: 'Midnight Signals',
    artist: 'Nora Lind',
    album: 'Northern Drift',
    sourceDevice: 'arch-workstation',
    playerName: 'MPRIS2',
    durationSec: 234,
    positionSec: 86,
    isPlaying: false,
  },
  {
    id: 'track-2',
    title: 'Glass Harbor',
    artist: 'Kaito Mori',
    album: 'Quiet Channels',
    sourceDevice: 'arch-workstation',
    playerName: 'MPRIS2',
    durationSec: 198,
    positionSec: 42,
    isPlaying: false,
  },
  {
    id: 'track-3',
    title: 'Static Bloom',
    artist: 'Sora Collective',
    album: 'Late Frequencies',
    sourceDevice: 'arch-workstation',
    playerName: 'MPRIS2',
    durationSec: 261,
    positionSec: 118,
    isPlaying: false,
  },
];

export const DEMO_MIRRORED_NOTIFICATION: MirroredNotification = {
  id: 'mirror-signal-1',
  appName: 'Signal',
  sender: 'Elena Popescu',
  body: 'Uploaded the Wayland capture notes to the shared folder.',
  timestamp: '22:53',
  read: false,
};

export const DEMO_CALL_ALERT: CallAlert = {
  id: 'call-1',
  callerName: 'Andrei Ionescu',
  phoneNumber: '+40 723 418 205',
  state: 'ringing',
  headsetProfile: 'hfp_hf',
};
