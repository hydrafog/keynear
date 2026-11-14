pub mod device_detail;
pub mod receive;
pub mod send;
pub mod settings;
pub mod shell;

use gtk4::prelude::*;

pub(crate) fn clear_box_children(container: &gtk4::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

pub(crate) fn sync_switch(sw: &gtk4::Switch, desired: bool) {
    if sw.is_active() != desired {
        sw.set_active(desired);
    }
}
