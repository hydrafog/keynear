# Smart Audio Handoff

`Keynear` coordinates shared Bluetooth audio peripherals (earbuds, headphones, and speakers) between Linux and Android without a mid-stream disconnect.

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

- `keynear-linux` monitors BlueZ device and transport objects alongside PipeWire (`bap_sink` for LE Audio, `a2dp_sink`, and `hfp_hf`) and MPRIS2 media players.
- Android publishes call and sink state over the continuity channel. Linux pauses active MPRIS2 playback and releases the PipeWire sink profile before the peer claims the headset, then reclaims the sink and resumes playback when it returns.
