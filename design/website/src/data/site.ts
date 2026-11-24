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
  readonly caption: string;
  readonly icon: LucideIcon;
}

export interface HeroScreenshot {
  readonly src: ThemedSource;
  readonly alt: string;
  readonly label: string;
}

export interface CapabilityItem {
  readonly id: 'files' | 'clipboard' | 'media' | 'calls';
  readonly src: string;
  readonly alt: string;
  readonly title: string;
  readonly detail: string;
}

export const LINKS = {
  github: 'https://github.com/hydrafog/keynear',
  issues: 'https://github.com/hydrafog/keynear/issues',
  androidReleases: 'https://github.com/hydrafog/keynear/releases',
  linuxReleases: 'https://github.com/hydrafog/keynear/releases',
} as const;

export const NAV_ITEMS: readonly NavItem[] = [
  { id: 'how-it-works', label: 'How it works' },
  { id: 'capabilities', label: 'Capabilities' },
  { id: 'screenshots', label: 'Screenshots' },
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

export const HERO_SCREENSHOT: HeroScreenshot = {
  src: {
    light: '/screenshots/receive-light.png',
    dark: '/screenshots/receive-dark.png',
  },
  alt: 'Keynear Android Receive screen showing nearby Linux computers',
  label: 'Hero screenshot',
};

export const SCREENSHOTS: readonly ScreenshotItem[] = [
  {
    id: 'receive',
    src: {
      light: '/screenshots/receive-light.png',
      dark: '/screenshots/receive-dark.png',
    },
    label: 'Receive screen',
    caption: 'Proximity discovery with Hidden, Paired, and Everyone nearby controls.',
    icon: Inbox,
  },
  {
    id: 'send',
    src: {
      light: '/screenshots/send-light.png',
      dark: '/screenshots/send-dark.png',
    },
    label: 'Send screen',
    caption: 'Queue files, folders, clipboard contents, or text for paired computers.',
    icon: Send,
  },
  {
    id: 'settings',
    src: {
      light: '/screenshots/settings-light.png',
      dark: '/screenshots/settings-dark.png',
    },
    label: 'Settings screen',
    caption: 'Permissions, save folder, clipboard sync, and appearance controls.',
    icon: SlidersHorizontal,
  },
];

export const CAPABILITIES: readonly CapabilityItem[] = [
  {
    id: 'files',
    src: '',
    alt: 'Wireframe illustration for direct file and folder drops',
    title: 'Direct file and folder drops',
    detail:
      'Queue files, folders, clipboard items, or notes from Android Direct Share. Paired computers auto-accept into Downloads/Keynear.',
  },
  {
    id: 'clipboard',
    src: '',
    alt: 'Wireframe illustration for clipboard sync and 2FA pass-through',
    title: 'Clipboard and 2FA pass-through',
    detail:
      'Sync text and images while skipping password-manager MIME types. Forward SMS verification codes to Linux with one tap.',
  },
  {
    id: 'media',
    src: '',
    alt: 'Wireframe illustration for mirrored notifications and desktop media controls',
    title: 'Mirrored notifications and media',
    detail:
      'Read selected Android notifications on Linux with inline replies, and control desktop MPRIS2 playback from the phone shade.',
  },
  {
    id: 'calls',
    src: '',
    alt: 'Wireframe illustration for call handoff and lock-screen safety',
    title: 'Call handoff and lock-screen safety',
    detail:
      'Route phone calls through your Linux PipeWire headset profile. Keep Quick Settings limited to Hidden and Paired while locked.',
  },
];
