pub mod adapter;
pub mod advertiser;
pub mod gatt;

pub use adapter::{BleDriverParams, BlePeerDriver};

use keynear_core::{CAP_PRESENCE_GATING, CAP_PROXIMITY_LOCK, CAP_RANGING_UWB};

pub const DEFAULT_CAPABILITY_FLAGS: u32 =
    0x07 | CAP_PROXIMITY_LOCK | CAP_RANGING_UWB | CAP_PRESENCE_GATING;

#[cfg(test)]
mod tests;
