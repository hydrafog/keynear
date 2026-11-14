# Clipboard and OTP Pass-Through

`Keynear` synchronizes copied text and images in the background while respecting OS privacy boundaries.

## Linux Clipboard Sync

- The daemon prefers `ext-data-control-v1` (KDE KWin, COSMIC, Sway, Hyprland) and falls back to `zwlr-data-control-v1` and X11. Outgoing images use delayed rendering.
- Incoming clipboard offers are inspected before any payload is read. Entries carrying `x-kde-passwordManagerHint` or `application/x-password` are skipped automatically.

## Android Clipboard and OTPs

- Clipboard items received from Linux land in the background. Reading the phone clipboard respects Android focus rules and runs from a notification or Quick Settings tile action.
- One-time passwords on Android use SMS User Consent, which presents a consent-driven prompt, copies the verified code to the desktop clipboard, and exposes it as the **Copy latest OTP** action in the tray menu.
