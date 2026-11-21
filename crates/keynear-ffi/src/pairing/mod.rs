use std::fmt;
use std::sync::Mutex;

use keynear_core::{
    derive_presence_token, EveryoneNearbyBeacon, LanEndpointCodec, NoisePairingSession,
    PairedBeacon, PairedBeaconState, PairingCompleted, PairingPeerMetadata, PairingRole,
    PairingState,
};
use keynear_protocol::{v1, DeviceRole};

use crate::callbacks::{to_key_32, FfiDeviceRole, FfiError, FfiLanEndpoint};

mod beacon;
mod session;
mod types;

pub(crate) use beacon::{
    decode_everyone_nearby_beacon_impl, decode_lan_endpoint_impl, decode_paired_beacon_impl,
    derive_presence_token_bytes_impl, encode_everyone_nearby_beacon_impl, encode_lan_endpoint_impl,
    encode_paired_beacon_impl,
};
pub use types::{
    FfiEveryoneNearbyBeacon, FfiNoisePairingSession, FfiPairedBeaconState, FfiPairingCompleted,
    FfiPairingRole, FfiPairingState, FfiPairingStep, FfiStaticKeypair,
};
