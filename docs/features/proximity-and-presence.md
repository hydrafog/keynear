# Proximity and Presence

Keynear (`1.3.0-phoenix`) locks the Linux desktop when a paired phone leaves desk range and pauses presence-gated continuity while the phone is away or the session is locked.

## Proximity Supervision

The paired phone measures distance with UWB on supported hardware running Android 16 and falls back to companion presence with smoothed BLE RSSI where background ranging is restricted.

- `crates/keynear-core/src/proximity.rs` owns `ProximityEvaluator`. It smooths RSSI with an exponential moving average and requires two consecutive samples beyond the configured bound before changing state. Entering range uses the stricter enter bound; leaving range uses the wider exit bound, so jitter near the threshold does not flap the lock state.
- `crates/keynear/src/proximity/lock.rs` calls `org.freedesktop.login1.Manager.LockSession` and `LockSessions` through `zbus` and reads `org.freedesktop.login1.Session.LockedHint`. A failed call becomes a typed `KeynearError` that is logged and retried on the next sample.
- `crates/keynear/src/service/pairing.rs` feeds paired-device RSSI from the BLE discovery stream into `crates/keynear/src/proximity/supervisor.rs`. The supervisor assigns a monotonic sequence to each sample and logs the resulting state and blocked operation count.
- `android/app/src/main/java/org/keynear/app/proximity/ProximitySupervisor.kt` checks `RangingManager` capabilities and starts a raw UWB session with a shared session key. When UWB is unavailable it reports the fallback path. `RangingCallback.kt` converts `RangingData` into `FfiRangingSample` records.
- `RangingReport` and `ProximityState` control frames carry distance, ranging technology, RSSI, and session-lock state at tags `35` and `36` of `keynear.v1.Envelope`.

## Presence Gating

`crates/keynear-core/src/presence.rs` holds `PresenceGate`, which denies clipboard sync, OTP forwarding, and automatic transfer acceptance while the phone is out of range, the session is locked, or presence is unknown. The gate resumes all three paths once the phone returns and the session unlocks. Missing measurements fail closed so an unattended desktop never receives a private payload.

## Configuration

`DaemonConfig` exposes `proximityLockEnabled`, `presenceGatingEnabled`, and `proximityDistanceMeters`. The existing Android settings surface adds a proximity lock switch, a presence gating switch, and a tap-to-adjust desk distance row. `RANGING` is declared in `AndroidManifest.xml` and belongs to the nearby-devices permission group already requested during onboarding.
