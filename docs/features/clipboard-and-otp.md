# Clipboard and OTP Pass-Through

`Keynear` synchronizes copied text and images in the background while respecting OS privacy boundaries.

## Linux Clipboard Sync

- **Wayland & X11**: The daemon prioritizes `ext-data-control-v1` (KDE KWin, COSMIC, Sway, Hyprland) with `zwlr-data-control-v1` and X11 fallbacks, using delayed rendering for outgoing images.
- **Password Exclusion**: Incoming clipboard offers are inspected before reading. Entries carrying `x-kde-passwordManagerHint` or `application/x-password` are skipped automatically.

## Android Clipboard and OTPs

- **Background Sync**: Receiving clipboard items from Linux works in the background. Reading the phone clipboard respects Android focus rules via notification or Quick Settings tile actions.
- **SMS User Consent**: One-time passwords on Android use SMS User Consent to present a consent-driven prompt that copies the verified code to the desktop clipboard and makes it available under **Copy latest OTP** in the tray menu.

