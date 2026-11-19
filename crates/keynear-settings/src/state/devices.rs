use keynear_protocol::{NearbyPeer, PairedDevice, VisibilityMode, PAIRING_TIMEOUT_SECS};

use super::SettingsModel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitPlacement {
    pub radius_px: i32,
    pub duration_secs: u64,
    pub phase_secs: u64,
    pub offset_x: i32,
    pub offset_y: i32,
}

pub fn orbit_for(rssi: i16, index: usize, count: usize) -> OrbitPlacement {
    let t = (f64::from(-rssi) - 36.0) / 56.0;
    let radius = 112.0 + t.clamp(0.0, 1.0) * 68.0;
    let angle_deg = (index as f64 / count.max(1) as f64) * 360.0;
    let angle_rad = (angle_deg - 90.0).to_radians();
    OrbitPlacement {
        radius_px: radius.round() as i32,
        duration_secs: 40,
        phase_secs: ((index as f64 / count.max(1) as f64) * 40.0).round() as u64,
        offset_x: (radius * angle_rad.cos()).round() as i32,
        offset_y: (radius * angle_rad.sin()).round() as i32,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPairingView {
    pub peer_token: String,
    pub peer_name: String,
    pub verification_code: String,
    pub remaining_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapPin {
    pub id: String,
    pub display_name: String,
    pub state_label: String,
    pub rssi: i16,
    pub is_paired: bool,
    pub is_online: bool,
}

impl SettingsModel {
    pub fn paired_devices(&self) -> &[PairedDevice] {
        &self.paired_devices
    }

    pub fn pending_pairing(&self) -> Option<&PendingPairingView> {
        self.pending_pairing.as_ref()
    }

    pub fn pending_unpair_id(&self) -> Option<&str> {
        self.pending_unpair_id.as_deref()
    }

    pub fn set_pending_pairing_for_test(&mut self, prompt: PendingPairingView) {
        self.pending_pairing = Some(prompt);
    }

    pub fn map_pins(&self) -> Vec<MapPin> {
        let mut pins = Vec::new();
        for (idx, dev) in self.paired_devices.iter().enumerate() {
            let online = !dev.last_lan_address.is_empty();
            pins.push(MapPin {
                id: dev.device_id.clone(),
                display_name: dev.display_name.clone(),
                state_label: if online { "Wi-Fi" } else { "Standby" }.to_string(),
                rssi: if online { -50 - (idx as i16 * 5) } else { -76 },
                is_paired: true,
                is_online: online,
            });
        }
        if self.visibility_mode != VisibilityMode::Hidden {
            for peer in &self.nearby_peers {
                if !peer.is_paired {
                    pins.push(MapPin {
                        id: peer.presence_token.clone(),
                        display_name: peer.display_name.clone(),
                        state_label: "Tap to pair".to_string(),
                        rssi: peer.rssi,
                        is_paired: false,
                        is_online: true,
                    });
                }
            }
        }
        pins
    }

    pub fn selected_detail_device(&self) -> Option<&PairedDevice> {
        match self.sub_screen {
            Some(super::SubScreen::DeviceDetail(ref device_id)) => self
                .paired_devices
                .iter()
                .find(|d| &d.device_id == device_id),
            _ => None,
        }
    }

    pub(super) fn begin_pair(&mut self, peer_token: &str) {
        if let Some(peer) = self
            .nearby_peers
            .iter()
            .find(|p| p.presence_token == peer_token)
        {
            self.pending_pairing = Some(build_pairing_prompt(peer));
            self.error_message = None;
        }
    }
}

fn build_pairing_prompt(peer: &NearbyPeer) -> PendingPairingView {
    let code_num: u32 = peer.presence_token.bytes().fold(100_007u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(u32::from(b))
    }) % 1_000_000;
    let raw = format!("{code_num:06}");
    PendingPairingView {
        peer_token: peer.presence_token.clone(),
        peer_name: peer.display_name.clone(),
        verification_code: format!("{} {}", &raw[..3], &raw[3..6]),
        remaining_secs: PAIRING_TIMEOUT_SECS,
    }
}
