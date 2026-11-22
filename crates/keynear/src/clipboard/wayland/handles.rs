use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_offer_v1, ext_data_control_source_v1,
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_offer_v1, zwlr_data_control_source_v1,
};

pub(super) enum ActiveOfferHandle {
    Ext(ext_data_control_offer_v1::ExtDataControlOfferV1),
    Zwlr(zwlr_data_control_offer_v1::ZwlrDataControlOfferV1),
}

impl ActiveOfferHandle {
    pub(super) fn receive(&self, mime: String, fd: std::os::fd::BorrowedFd<'_>) {
        match self {
            Self::Ext(o) => o.receive(mime, fd),
            Self::Zwlr(o) => o.receive(mime, fd),
        }
    }

    pub(super) fn destroy(&self) {
        match self {
            Self::Ext(o) => o.destroy(),
            Self::Zwlr(o) => o.destroy(),
        }
    }
}

pub(super) enum ActiveSourceHandle {
    Ext(ext_data_control_source_v1::ExtDataControlSourceV1),
    Zwlr(zwlr_data_control_source_v1::ZwlrDataControlSourceV1),
}

impl ActiveSourceHandle {
    pub(super) fn destroy(&self) {
        match self {
            Self::Ext(s) => s.destroy(),
            Self::Zwlr(s) => s.destroy(),
        }
    }
}
