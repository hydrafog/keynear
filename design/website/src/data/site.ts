import type { LucideIcon } from 'lucide-react';
import { Inbox, Send, SlidersHorizontal } from 'lucide-react';

export type ThemeMode = 'light' | 'dark';

export interface SlideItem {
  readonly id: 'overview' | 'how-it-works' | 'privacy';
  readonly index: number;
  readonly step: string;
  readonly label: string;
}

export interface PrivacyItem {
  readonly id: 'telemetry' | 'keys' | 'memory' | 'permissions';
  readonly title: string;
  readonly detail: string;
}

export interface PairingStep {
  readonly step: string;
  readonly title: string;
  readonly detail: string;
}

export interface ThemedSource {
  readonly light: string;
  readonly dark: string;
}

export interface ScreenshotItem {
  readonly id: 'receive' | 'send' | 'settings';
  readonly src: ThemedSource;
  readonly label: string;
  readonly icon: LucideIcon;
}

export interface CapabilityItem {
  readonly id: 'files' | 'clipboard' | 'media' | 'calls';
  readonly src: string;
  readonly alt: string;
  readonly title: string;
  readonly detail: string;
}

const ASSET_BASE = import.meta.env.BASE_URL;

export const LINKS = {
  github: 'https://github.com/hydrafog/keynear',
  issues: 'https://github.com/hydrafog/keynear/issues',
  releases: 'https://github.com/hydrafog/keynear/releases',
} as const;

export const SLIDES: readonly SlideItem[] = [
  { id: 'overview', index: 0, step: '01', label: 'Overview' },
  { id: 'how-it-works', index: 1, step: '02', label: 'How it works' },
  { id: 'privacy', index: 2, step: '03', label: 'Privacy' },
];

export const PRIVACY_ITEMS: readonly PrivacyItem[] = [
  {
    id: 'telemetry',
    title: 'Zero telemetry or cloud',
    detail:
      'Keynear collects no analytics, usage metrics, or crash reports. Discovery and transfers stay on BLE and local Wi-Fi QUIC.',
  },
  {
    id: 'keys',
    title: 'Hardware-backed secrets',
    detail:
      'X25519, Ed25519, and pairing keys stay inside Android Keystore and Linux Secret Service. Unpairing wipes metadata and secrets.',
  },
  {
    id: 'memory',
    title: 'Memory-only continuity',
    detail:
      'Mirrored notifications, call numbers, media metadata, and OTP codes are never written to disk, config files, or logs.',
  },
  {
    id: 'permissions',
    title: 'Minimal OS permissions',
    detail:
      'Never requests fine location, READ_SMS, or broad storage access. Password-manager and sensitive clipboard MIME types are skipped.',
  },
];

export const PAIRING_STEPS: readonly PairingStep[] = [
  {
    step: '01',
    title: 'Discover over Bluetooth',
    detail: 'BLE finds your nearby Linux desktop.',
  },
  {
    step: '02',
    title: 'Confirm 6-digit code',
    detail: 'Verify once to pin X25519 keys on device.',
  },
  {
    step: '03',
    title: 'Bridge over Wi-Fi',
    detail: 'Everything moves directly over your LAN.',
  },
];

export const SCREENSHOTS: readonly ScreenshotItem[] = [
  {
    id: 'receive',
    src: {
      light: `${ASSET_BASE}screenshots/receive-light.png`,
      dark: `${ASSET_BASE}screenshots/receive-dark.png`,
    },
    label: 'Receive screen',
    icon: Inbox,
  },
  {
    id: 'send',
    src: {
      light: `${ASSET_BASE}screenshots/send-light.png`,
      dark: `${ASSET_BASE}screenshots/send-dark.png`,
    },
    label: 'Send screen',
    icon: Send,
  },
  {
    id: 'settings',
    src: {
      light: `${ASSET_BASE}screenshots/settings-light.png`,
      dark: `${ASSET_BASE}screenshots/settings-dark.png`,
    },
    label: 'Settings screen',
    icon: SlidersHorizontal,
  },
];

export const CAPABILITIES: readonly CapabilityItem[] = [
  {
    id: 'files',
    src: `${ASSET_BASE}wireframes/files.png`,
    alt: 'Wireframe illustration for file and folder drops',
    title: 'File & folder drops',
    detail: 'Share files or folders straight into Downloads/Keynear.',
  },
  {
    id: 'clipboard',
    src: `${ASSET_BASE}wireframes/clipboard.png`,
    alt: 'Wireframe illustration for clipboard sync and 2FA codes',
    title: 'Clipboard & 2FA',
    detail: 'Sync text and images, skip passwords, forward SMS codes.',
  },
  {
    id: 'media',
    src: `${ASSET_BASE}wireframes/media.png`,
    alt: 'Wireframe illustration for notifications and desktop media controls',
    title: 'Notifications & media',
    detail: 'Reply to Android alerts on Linux and control desktop playback.',
  },
  {
    id: 'calls',
    src: `${ASSET_BASE}wireframes/calls.png`,
    alt: 'Wireframe illustration for call handoff and lock-screen safety',
    title: 'Calls & lock-screen safety',
    detail: 'Route calls to PipeWire headsets; stay hidden while locked.',
  },
];
