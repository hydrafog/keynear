export type VisibilityMode = 'hidden' | 'pairedOnly' | 'everyoneNearby';

export interface DaemonConfig {
  visibilityMode: VisibilityMode;
  incomingDirectory: string;
  autoAcceptFromPaired: boolean;
  clipboardSyncEnabled: boolean;
  clipboardImagesEnabled: boolean;
  otpPassthroughEnabled: boolean;
  audioHandoffEnabled: boolean;
  ignoredMimeTypes: string[];
  notifyAutoAcceptComplete: boolean;
  notificationMirrorEnabled: boolean;
}

export type PermissionKey =
  | 'nearbyDevices'
  | 'localNetwork'
  | 'notifications'
  | 'companionAssociation'
  | 'batteryUnrestricted';

export type PermissionsState = Record<PermissionKey, boolean>;

export interface PairedDevice {
  deviceId: string;
  displayName: string;
  staticX25519Pubkey: string;
  lastBleAddress: string;
  lastLanAddress: string;
  autoAcceptFiles: boolean;
  pairedAt: string;
  isOnline: boolean;
  connectionLabel: string;
  rssi: number;
}

export interface NearbyPeer {
  presenceToken: string;
  displayName: string;
  rssi: number;
}

export interface TransferItem {
  name: string;
  sizeBytes: number;
  isDirectory: boolean;
}

export interface SendQueueItem extends TransferItem {
  id: string;
}

export type TransferDirection = 'send' | 'receive';

export type TransferState = 'pendingConsent' | 'active' | 'completed' | 'cancelled';

export interface TransferSession {
  transferId: string;
  peerDeviceId: string;
  peerName: string;
  direction: TransferDirection;
  items: TransferItem[];
  totalBytes: number;
  transferredBytes: number;
  state: TransferState;
}

export interface PairingPrompt {
  peerName: string;
  verificationCode: string;
  pubkeyFingerprint: string;
  bleAddress: string;
  secondsLeft: number;
}

export interface OtpPrompt {
  sender: string;
  code: string;
  targetDesktop: string;
}

export interface MediaTrack {
  id: string;
  title: string;
  artist: string;
  album: string;
  sourceDevice: string;
  playerName: string;
  durationSec: number;
  positionSec: number;
  isPlaying: boolean;
}

export interface MirroredNotification {
  id: string;
  appName: string;
  sender: string;
  body: string;
  timestamp: string;
  read: boolean;
  replySent?: string;
}

export interface CallAlert {
  id: string;
  callerName: string;
  phoneNumber: string;
  state: 'ringing' | 'active' | 'muted';
  headsetProfile: string;
}

export type ActiveTab = 'receive' | 'send' | 'settings';

export type SubScreen = 'deviceDetail' | 'permissions' | null;
