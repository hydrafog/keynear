pub mod adapter;
pub mod advertiser;
pub mod gatt;

pub use adapter::{BleDriverParams, BlePeerDriver};

pub const DEFAULT_CAPABILITY_FLAGS: u32 = 0x07;

#[cfg(test)]
mod tests;
