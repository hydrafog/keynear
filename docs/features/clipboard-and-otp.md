# Clipboard and OTP Pass-Through

`Keynear` (`0.5.0-draco`) synchronizes copied text and images in the background while respecting OS privacy boundaries.

## Linux Clipboard Sync

- The daemon (`crates/keynear-linux/src/clipboard.rs`) prefers `ext-data-control-v1` (KDE KWin, COSMIC, Sway, Hyprland) and falls back to `zwlr-data-control-v1` and X11 (`x11rb` XFixes), degrading non-fatally when headless. Outgoing images advertise `ClipboardOffer` metadata first and render payload bytes lazily over `QuicStreamKind::ClipboardBlob` (`0x04`) when a paired peer requests `ClipboardFetch`.
- Clipboard offers are inspected before any payload byte is read. Entries carrying `x-kde-passwordManagerHint`, `application/x-password`, or user-configured `ignoredMimeTypes` are skipped automatically, and `keynear-settings` propagates `Clipboard sync`, `Sync clipboard images`, `2FA code pass-through`, and `Ignored clipboard formats` (`MAX_IPC_MIMES = 128`) live through `UpdateConfig` IPC without restarting the daemon.
- Self-authored selection writes are suppressed over a `5`-second SHA-256 digest window (`SelfWriteTracker`) so peer updates never echo back.

## Android Clipboard and OTPs

- Clipboard items received from Linux land on the phone clipboard in the background through `KeynearForegroundService` (`CoreClient.syncClipboardStates`). Reading the phone clipboard respects Android 10+ input-focus rules by launching a focused translucent `ClipboardReadActivity` from the **Copy phone clipboard** ongoing notification action or the **Copy phone clipboard** Quick Settings tile (`ClipboardTileService`).
- One-time passwords on Android use Google Play Services SMS User Consent (`SmsRetrieverClient.startSmsUserConsent`) gated by `otpPassthroughEnabled`, `SmsRetriever.SEND_PERMISSION`, `com.google.android.gms` consent intent verification, and `Activity.RESULT_OK` (posting a status notification when Play Services is absent). Verified codes forward as `ClipboardPayload { is_otp: true }` to the desktop clipboard and stay strictly in daemon RAM (`latest_otp`) for the **Copy latest OTP** action in the `ksni` tray menu.
