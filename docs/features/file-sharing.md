# File Sharing

`Keynear` (`0.4.0-aquila`) streams files and directories directly between paired peers over relay-free `iroh` QUIC streams.

## Transfer Pipeline

```mermaid
flowchart LR
  src["Source File or Folder"] --> chunk["60 KiB Chunks or STORED Zip"]
  chunk --> quic["Direct iroh QUIC Stream (X25519MLKEM768)"]
  quic --> consent["PendingConsent or autoAcceptFiles"]
  consent --> linuxDst["Linux ~/Downloads/Keynear"]
  consent --> androidDst["Android MediaStore.Downloads"]
```

## Streaming Mechanics

- Individual files stream in 60 KiB (`61_440`-byte, `STREAM_CHUNK_BYTES`) chunks over `QuicStreamKind::RawFileChunk`. On Android, `CoreClient` inserts a `MediaStore.Downloads` row with `IS_PENDING = 1`, passes `ParcelFileDescriptor.detachFd()` (`OwnedFd` in Rust) to `OrionNodeHandle.provide_receive_fd` or `send_fd` without a JVM heap copy, and clears `IS_PENDING = 0` on completion or deletes the pending row on error or cancellation.
- Directories and multi-file batches stream as uncompressed `CompressionMethod::Stored` zip archives (`zip::ZipWriter::new_stream`) over `QuicStreamKind::StoredZipArchive` using `1 MiB` (`1_048_576`-byte, `ZIP_IO_BUFFER_BYTES`) buffered I/O. Senders skip symlinks (`symlink_metadata`). Receivers spool and validate SHA-256 against `TransferComplete` before unpacking via `ZipArchive` into `incoming_directory` (default `~/Downloads/Keynear` on Linux), rejecting path traversal (`enclosed_name()`), symlinks, encrypted entries, non-`Stored` entries, and byte counts exceeding `TransferOffer.total_bytes`.

## Transfer Consent and Auto-Accept

- Incoming `TransferOffer` messages from authenticated paired devices enter `PendingConsent` by default and do not write target files until approved (`TransferConsent(accepted = true)`), transitioning through `Active` to `Completed`, `Cancelled`, or `Failed`.
- Per-device `PairedDevice.auto_accept_files` overrides and global `DaemonConfig.auto_accept_from_paired` (both `false` by default) bypass the consent prompt and transition paired offers straight to `Active`. Declining an offer returns `TransferConsent(accepted = false)` without touching target storage.

## Desktop and Mobile Entrypoints

- **Linux daemon, tray, and notifications**: `keynear-linux` executes `--send <device_id> <path>...`, `--accept <transfer_id>`, `--decline <transfer_id>`, `--cancel <transfer_id>`, and `--transfers` over the `0o600` `daemon.sock` Unix socket. When a graphical D-Bus session is available, `keynear-linux` registers a `ksni` `StatusNotifierItem` tray menu and posts `org.freedesktop.Notifications` consent and status alerts with inline actions.
- **Linux settings portal and file-manager integrations**: `keynear-settings` uses `ashpd` (`xdg-desktop-portal`) file and folder choosers to send files, toggle global `auto_accept_from_paired` or per-device `Auto-accept files`, rename paired devices inline, and manage `Active transfers` (`Accept`, `Decline`, `Cancel`). `--send-picker [--device <id>]` and `--share [--device <id>] [--] <path>...` power the Nautilus (`integrations/nautilus/send-via-keynear`), Dolphin (`integrations/dolphin/keynear-send.desktop`), and Yazi (`integrations/yazi/keynear.yazi/main.lua`) context-menu integrations.
- **Android UI, Direct Share, and Live Updates**: `PairingNotificationManager` posts incoming consent prompts (`Accept`/`Decline`), completion/error alerts, and ongoing progress notifications rendered with Android 16 (API 36) `Notification.ProgressStyle` three-segment progress (`Prepare`, `Transfer`, `Complete`) or API 31–35 `NotificationCompat` progress bars backed by `KeynearTransferJobService` (`User-Initiated Data Transfer Jobs`). `MainActivity` exposes `Active transfers`, paired-device rename, and per-device `Auto-accept files` switches; `ShareActivity` renders a translucent bottom-sheet picker synchronized with `DirectShareManager` Direct Share targets (`res/xml/share_targets.xml`).
