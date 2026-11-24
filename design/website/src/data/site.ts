import type { LucideIcon } from 'lucide-react';
import { Inbox, Send, SlidersHorizontal } from 'lucide-react';

export type ThemeMode = 'light' | 'dark';

export interface NavItem {
  readonly id: string;
  readonly label: string;
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

export const NAV_ITEMS: readonly NavItem[] = [
  { id: 'how-it-works', label: 'How it works' },
  { id: 'capabilities', label: 'Capabilities' },
];

export const PAIRING_STEPS: readonly PairingStep[] = [
  {
    step: '01',
    title: 'Discover over Bluetooth',
    detail:
      'Bluetooth Low Energy announces nearby presence when your phone and Linux desktop share the room.',
  },
  {
    step: '02',
    title: 'Confirm a 6-digit code',
    detail:
      'Approve the matching 6-digit code within 120 seconds to pin static X25519 keys in Android Keystore.',
  },
  {
    step: '03',
    title: 'Bridge over local Wi-Fi',
    detail:
      'Files, clipboard items, notifications, and media controls move directly over your local network with no relay server.',
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
    alt: 'Wireframe illustration for direct file and folder drops',
    title: 'Direct file and folder drops',
    detail:
      'Queue files, folders, clipboard items, or notes from Android Direct Share. Paired computers auto-accept into Downloads/Keynear.',
  },
  {
    id: 'clipboard',
    src: `${ASSET_BASE}wireframes/clipboard.png`,
    alt: 'Wireframe illustration for clipboard sync and 2FA pass-through',
    title: 'Clipboard and 2FA pass-through',
    detail:
      'Sync text and images while skipping password-manager MIME types. Forward SMS verification codes to Linux with one tap.',
  },
  {
    id: 'media',
    src: `${ASSET_BASE}wireframes/media.png`,
    alt: 'Wireframe illustration for mirrored notifications and desktop media controls',
    title: 'Mirrored notifications and media',
    detail:
      'Read selected Android notifications on Linux with inline replies, and control desktop MPRIS2 playback from the phone shade.',
  },
  {
    id: 'calls',
    src: `${ASSET_BASE}wireframes/calls.png`,
    alt: 'Wireframe illustration for call handoff and lock-screen safety',
    title: 'Call handoff and lock-screen safety',
    detail:
      'Route phone calls through your Linux PipeWire headset profile. Keep Quick Settings limited to Hidden and Paired while locked.',
  },
];
