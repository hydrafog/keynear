mod basic;
mod files;
mod status;

use gtk4::cairo;

use super::LucideIcon;

pub(crate) fn draw_lucide_path(cr: &cairo::Context, icon: LucideIcon) {
    match icon {
        LucideIcon::ArrowLeft
        | LucideIcon::ChevronRight
        | LucideIcon::ChevronDown
        | LucideIcon::ChevronUp => basic::draw_nav_icons(cr, icon),
        LucideIcon::EyeOff => status::draw_eye_off(cr),
        LucideIcon::ShieldCheck => status::draw_shield_check(cr),
        LucideIcon::Radio => status::draw_radio(cr),
        LucideIcon::Inbox => status::draw_inbox(cr),
        LucideIcon::Send => basic::draw_send(cr),
        LucideIcon::Laptop | LucideIcon::Smartphone => basic::draw_devices(cr, icon),
        LucideIcon::FileText => files::draw_file_text(cr),
        LucideIcon::Folder => files::draw_folder(cr),
        LucideIcon::Clipboard => files::draw_clipboard(cr),
        LucideIcon::Plus | LucideIcon::X => basic::draw_plus_x(cr, icon),
        LucideIcon::RefreshCw => status::draw_refresh_cw(cr),
        LucideIcon::Trash2 => status::draw_trash2(cr),
        LucideIcon::SlidersHorizontal => basic::draw_sliders(cr),
    }
}
