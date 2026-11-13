# Smart Audio Handoff

`Keynear` coordinates shared Bluetooth audio peripherals (earbuds, headphones, and speakers) across Linux and Android without mid-stream disconnects.

## Handoff Sequence

```mermaid
sequenceDiagram
  participant A as Android App
  participant L as Linux Daemon
  participant P as PipeWire & MPRIS2
  A->>L: Publish call or sink claim over QUIC
  L->>P: Pause active MPRIS2 players
  L->>P: Release bap_sink / a2dp_sink profile
  A-->>A: Claim Bluetooth audio peripheral
  A->>L: Publish sink release
  L->>P: Reclaim sink and resume playback
```

## Coordination Details

- **Linux Monitoring**: `keynear-linux` monitors BlueZ device/transport objects alongside PipeWire (`bap_sink` for LE Audio and `a2dp_sink` / `hfp_hf`) and MPRIS2 media players.
- **Conflict-Free Switching**: Android publishes call and sink state over the continuity channel while Linux pauses active MPRIS2 playback and releases the PipeWire sink profile cleanly before the peer claims the headset, resuming playback when the sink returns.
