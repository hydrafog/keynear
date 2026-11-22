use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use keynear_core::{ClipboardPolicy, DaemonConfig};
use tokio::sync::{mpsc, watch};
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_seat;
use wayland_client::QueueHandle;
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1, ext_data_control_manager_v1,
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1, zwlr_data_control_manager_v1,
};

use super::super::handle::BackendCmd;
use super::super::tracker::SelfWriteTracker;
use super::super::{ClipboardBackendKind, ClipboardChange};
use super::handles::{ActiveOfferHandle, ActiveSourceHandle};

pub(crate) struct WaylandAppState {
    pub(super) seat: Option<wl_seat::WlSeat>,
    pub(super) ext_manager: Option<ext_data_control_manager_v1::ExtDataControlManagerV1>,
    pub(super) zwlr_manager: Option<zwlr_data_control_manager_v1::ZwlrDataControlManagerV1>,
    pub(super) ext_device: Option<ext_data_control_device_v1::ExtDataControlDeviceV1>,
    pub(super) zwlr_device: Option<zwlr_data_control_device_v1::ZwlrDataControlDeviceV1>,
    pub(super) backend_kind: Option<ClipboardBackendKind>,
    pub(super) pending_offers: HashMap<ObjectId, Vec<String>>,
    pub(super) current_offer: Option<(u64, ActiveOfferHandle)>,
    pub(super) next_offer_seq: u64,
    pub(super) active_sources: HashMap<ObjectId, (ActiveSourceHandle, Arc<Vec<u8>>)>,
    pub(super) config_rx: watch::Receiver<DaemonConfig>,
    pub(super) event_tx: mpsc::UnboundedSender<ClipboardChange>,
    pub(super) self_writes: Arc<Mutex<SelfWriteTracker>>,
    pub(super) cmd_tx: mpsc::UnboundedSender<BackendCmd>,
}

impl WaylandAppState {
    pub(crate) fn new(
        config_rx: watch::Receiver<DaemonConfig>,
        event_tx: mpsc::UnboundedSender<ClipboardChange>,
        self_writes: Arc<Mutex<SelfWriteTracker>>,
        cmd_tx: mpsc::UnboundedSender<BackendCmd>,
    ) -> Self {
        Self {
            seat: None,
            ext_manager: None,
            zwlr_manager: None,
            ext_device: None,
            zwlr_device: None,
            backend_kind: None,
            pending_offers: HashMap::new(),
            current_offer: None,
            next_offer_seq: 1,
            active_sources: HashMap::new(),
            config_rx,
            event_tx,
            self_writes,
            cmd_tx,
        }
    }

    pub(crate) fn bind_preferred_device(
        &mut self,
        qh: &QueueHandle<Self>,
    ) -> Option<ClipboardBackendKind> {
        let seat = self.seat.as_ref()?;
        if let Some(ref mgr) = self.ext_manager {
            let dev = mgr.get_data_device(seat, qh, ());
            self.ext_device = Some(dev);
            self.backend_kind = Some(ClipboardBackendKind::ExtDataControlV1);
            return Some(ClipboardBackendKind::ExtDataControlV1);
        }
        if let Some(ref mgr) = self.zwlr_manager {
            let dev = mgr.get_data_device(seat, qh, ());
            self.zwlr_device = Some(dev);
            self.backend_kind = Some(ClipboardBackendKind::ZwlrDataControlV1);
            return Some(ClipboardBackendKind::ZwlrDataControlV1);
        }
        None
    }

    pub(super) fn current_policy(&self) -> ClipboardPolicy {
        let cfg = self.config_rx.borrow();
        ClipboardPolicy::from(&*cfg)
    }

    pub(super) fn on_data_offer(&mut self, offer_id: ObjectId) {
        self.pending_offers.insert(offer_id, Vec::new());
    }

    pub(super) fn on_offer_mime(&mut self, offer_id: &ObjectId, mime_type: String) {
        if let Some(list) = self.pending_offers.get_mut(offer_id) {
            list.push(mime_type);
        }
    }
}
