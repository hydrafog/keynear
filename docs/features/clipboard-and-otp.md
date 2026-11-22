# Clipboard and OTP Pass-Through

Keynear (`1.2.0-carina`) synchronizes copied text and images in the background while respecting OS privacy boundaries.

## Linux Clipboard Sync

The Linux daemon handles clipboard sync as follows.

- The daemon (`crates/keynear/src/clipboard.rs`) prefers `ext-data-control-v1` (KDE KWin, COSMIC, Sway, Hyprland). It falls back to `zwlr-data-control-v1` and X11 (`x11rb` XFixes), and degrades non-fatally when headless. Outgoing images advertise `ClipboardOffer` metadata first. When a paired peer requests `ClipboardFetch`, the daemon renders the payload bytes lazily over `QuicStreamKind::ClipboardBlob` (`0x04`).
- Keynear inspects clipboard offers before reading any payload byte. It skips entries carrying `x-kde-passwordManagerHint`, `application/x-password`, or user-configured `ignoredMimeTypes`. Configure four clipboard settings in `~/.config/keynear/config.toml` (checked with `keynear settings validate`) or update them live over `daemon.sock` through `IpcRequest::UpdateConfig`:
  - `clipboardSyncEnabled`.
  - `clipboardImagesEnabled`.
  - `otpPassthroughEnabled`.
  - `ignoredMimeTypes` (`MAX_IPC_MIMES = 128`).

  The daemon does not need a restart for these changes.
- A `5`-second SHA-256 digest window (`SelfWriteTracker`) suppresses self-authored selection writes. Peer updates never echo back.

## Android Clipboard and OTPs

Android clipboard and one-time-code handling works as follows.

- Clipboard items received from Linux land on the phone clipboard in the background through `KeynearForegroundService` (`CoreClient.syncClipboardStates`). Reading the phone clipboard respects Android 10+ input-focus rules. The **Copy phone clipboard** ongoing notification action or Quick Settings tile (`ClipboardTileService`) launches a focused translucent `ClipboardReadActivity`.
- One-time passwords on Android use Google Play Services SMS User Consent (`SmsRetrieverClient.startSmsUserConsent`). Four checks gate the feature.
  - `otpPassthroughEnabled`.
  - `SmsRetriever.SEND_PERMISSION`.
  - `com.google.android.gms` consent intent verification.
  - `Activity.RESULT_OK`.

  Keynear posts a status notification when Play Services is absent. Verified codes forward as `ClipboardPayload { is_otp: true }` to the desktop clipboard. They stay strictly in daemon RAM (`latest_otp`) for the **Copy latest OTP** action in the `ksni` tray menu.
