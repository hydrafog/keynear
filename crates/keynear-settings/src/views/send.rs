mod build;
mod rows;

use crate::views::components::LucideIcon;

pub(crate) use build::build_send_tab;
pub(crate) use rows::{rebuild_send_queue_rows, rebuild_send_targets, SendTargetsParams};

pub(crate) struct SendTabParts {
    pub(crate) root: gtk4::ScrolledWindow,
    pub(crate) text_tile: gtk4::Button,
    pub(crate) text_composer_box: gtk4::Box,
    pub(crate) text_entry: gtk4::Entry,
    pub(crate) queue_card: gtk4::Box,
    pub(crate) queue_heading: gtk4::Label,
    pub(crate) clear_btn: gtk4::Button,
    pub(crate) queue_list_box: gtk4::Box,
    pub(crate) error_banner: gtk4::Box,
    pub(crate) error_label: gtk4::Label,
    pub(crate) targets_box: gtk4::Box,
}
