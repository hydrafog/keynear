pub mod controller;
pub(crate) mod dbus;
pub(crate) mod pipewire;

pub use controller::{DbusHandoffControl, VegaAudioController};
pub use dbus::observe_sink_profile;
pub(crate) use dbus::{call_players, set_vcp_volume, switch_profile, vcp_volume};

use keynear_core::AudioSinkProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkProfileObservation {
    pub object_path: String,
    pub profile_name: String,
    pub profile: AudioSinkProfile,
}

impl SinkProfileObservation {
    pub fn none() -> Self {
        Self {
            object_path: String::new(),
            profile_name: String::new(),
            profile: AudioSinkProfile::BapSink,
        }
    }
}
