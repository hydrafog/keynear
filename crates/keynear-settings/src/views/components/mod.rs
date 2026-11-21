mod icons;

use gtk4::cairo;
use gtk4::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LucideIcon {
    ArrowLeft,
    Inbox,
    Send,
    SlidersHorizontal,
    EyeOff,
    ShieldCheck,
    Radio,
    Laptop,
    Smartphone,
    ChevronRight,
    ChevronDown,
    ChevronUp,
    FileText,
    Folder,
    Clipboard,
    Plus,
    X,
    RefreshCw,
    Trash2,
}

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

pub(crate) fn lucide_icon(icon: LucideIcon, size: i32) -> gtk4::DrawingArea {
    let area = gtk4::DrawingArea::builder()
        .content_width(size)
        .content_height(size)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();
    area.set_draw_func(move |widget, cr, w, h| {
        #[allow(deprecated)]
        let color = widget.style_context().color();
        cr.set_source_rgba(
            f64::from(color.red()),
            f64::from(color.green()),
            f64::from(color.blue()),
            f64::from(color.alpha()),
        );
        let dim = f64::from(w.min(h)).max(1.0);
        let scale = dim / 24.0;
        cr.save().ok();
        cr.translate((f64::from(w) - dim) * 0.5, (f64::from(h) - dim) * 0.5);
        cr.scale(scale, scale);
        cr.set_line_width(2.0);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);
        icons::draw_lucide_path(cr, icon);
        cr.restore().ok();
    });
    area
}
