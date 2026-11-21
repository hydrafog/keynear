mod build;
mod rows;

use keynear_protocol::{TransferDirection, TransferSession, TransferState};

use crate::views::components::LucideIcon;

pub(crate) use build::build_receive_sheets;
pub(crate) use rows::{rebuild_sheet_rows, rebuild_transfers_rows};

pub(crate) struct ReceiveSheetsParts {
    pub(crate) transfers_card: gtk4::Box,
    pub(crate) transfers_error_banner: gtk4::Box,
    pub(crate) transfers_error_label: gtk4::Label,
    pub(crate) transfers_list_box: gtk4::Box,
    pub(crate) sheet_card: gtk4::Box,
    pub(crate) sheet_count_label: gtk4::Label,
    pub(crate) sheet_toggle_label: gtk4::Label,
    pub(crate) sheet_scroll: gtk4::ScrolledWindow,
    pub(crate) sheet_list_box: gtk4::Box,
}

pub(crate) fn transfer_fraction(session: &TransferSession) -> f64 {
    if session.total_bytes == 0 {
        return match session.state {
            TransferState::Completed => 1.0,
            _ => 0.0,
        };
    }
    (session.transferred_bytes as f64 / session.total_bytes as f64).clamp(0.0, 1.0)
}

pub(crate) fn transfer_summary(session: &TransferSession) -> String {
    let dir = match session.direction {
        TransferDirection::Send => "To",
        TransferDirection::Receive => "From",
    };
    let state = match session.state {
        TransferState::PendingConsent => "Awaiting decision",
        TransferState::Active => "Transferring",
        TransferState::Completed => "Completed",
        TransferState::Cancelled => "Cancelled",
        TransferState::Failed => "Failed",
    };
    let title = session
        .items
        .first()
        .map(|i| i.name.as_str())
        .unwrap_or("Transfer");
    format!(
        "{title} ({} items, {state} · {dir} {})",
        session.items.len(),
        session.peer_device_id
    )
}
