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
        draw_lucide_path(cr, icon);
        cr.restore().ok();
    });
    area
}

fn draw_rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let d = std::f64::consts::FRAC_PI_2;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -d, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, d);
    cr.arc(x + r, y + h - r, r, d, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * d);
    cr.close_path();
    let _ = cr.stroke();
}

fn draw_lucide_path(cr: &cairo::Context, icon: LucideIcon) {
    match icon {
        LucideIcon::ArrowLeft => {
            cr.move_to(12.0, 19.0);
            cr.line_to(5.0, 12.0);
            cr.line_to(12.0, 5.0);
            let _ = cr.stroke();
            cr.move_to(19.0, 12.0);
            cr.line_to(5.0, 12.0);
            let _ = cr.stroke();
        }
        LucideIcon::Inbox => {
            cr.move_to(22.0, 12.0);
            cr.line_to(16.0, 12.0);
            cr.line_to(14.0, 15.0);
            cr.line_to(10.0, 15.0);
            cr.line_to(8.0, 12.0);
            cr.line_to(2.0, 12.0);
            let _ = cr.stroke();
            cr.move_to(5.45, 5.11);
            cr.line_to(2.0, 12.0);
            cr.line_to(2.0, 18.0);
            cr.arc(
                4.0,
                18.0,
                2.0,
                std::f64::consts::PI,
                std::f64::consts::FRAC_PI_2,
            );
            cr.line_to(20.0, 20.0);
            cr.arc(20.0, 18.0, 2.0, std::f64::consts::FRAC_PI_2, 0.0);
            cr.line_to(22.0, 12.0);
            cr.line_to(18.55, 5.11);
            cr.close_path();
            let _ = cr.stroke();
        }
        LucideIcon::Send => {
            cr.move_to(22.0, 2.0);
            cr.line_to(11.0, 13.0);
            let _ = cr.stroke();
            cr.move_to(22.0, 2.0);
            cr.line_to(15.0, 22.0);
            cr.line_to(11.0, 13.0);
            cr.line_to(2.0, 9.0);
            cr.close_path();
            let _ = cr.stroke();
        }
        LucideIcon::SlidersHorizontal => {
            for (y, kx) in [(4.0, 9.0), (12.0, 15.0), (20.0, 11.0)] {
                cr.move_to(3.0, y);
                cr.line_to(kx - 2.0, y);
                let _ = cr.stroke();
                cr.move_to(kx + 2.0, y);
                cr.line_to(21.0, y);
                let _ = cr.stroke();
                cr.move_to(kx, y - 2.5);
                cr.line_to(kx, y + 2.5);
                let _ = cr.stroke();
            }
        }
        LucideIcon::EyeOff => {
            cr.move_to(2.0, 2.0);
            cr.line_to(22.0, 22.0);
            let _ = cr.stroke();
            cr.move_to(6.7, 6.7);
            cr.curve_to(4.3, 8.3, 2.8, 10.6, 2.0, 12.0);
            cr.curve_to(3.7, 15.3, 7.5, 19.0, 12.0, 19.0);
            cr.curve_to(13.9, 19.0, 15.7, 18.3, 17.3, 17.3);
            let _ = cr.stroke();
            cr.move_to(10.6, 5.1);
            cr.curve_to(11.1, 5.0, 11.5, 5.0, 12.0, 5.0);
            cr.curve_to(16.5, 5.0, 20.3, 8.7, 22.0, 12.0);
            cr.curve_to(21.5, 13.0, 20.6, 14.3, 19.4, 15.5);
            let _ = cr.stroke();
        }
        LucideIcon::ShieldCheck => {
            cr.move_to(12.0, 2.0);
            cr.line_to(4.0, 5.5);
            cr.line_to(4.0, 11.5);
            cr.curve_to(4.0, 16.5, 7.4, 20.2, 12.0, 22.0);
            cr.curve_to(16.6, 20.2, 20.0, 16.5, 20.0, 11.5);
            cr.line_to(20.0, 5.5);
            cr.close_path();
            let _ = cr.stroke();
            cr.move_to(9.0, 12.0);
            cr.line_to(11.2, 14.2);
            cr.line_to(15.2, 10.0);
            let _ = cr.stroke();
        }
        LucideIcon::Radio => {
            cr.arc(12.0, 12.0, 2.0, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
            cr.arc(12.0, 12.0, 6.0, -0.75, 0.75);
            let _ = cr.stroke();
            cr.arc(
                12.0,
                12.0,
                6.0,
                std::f64::consts::PI - 0.75,
                std::f64::consts::PI + 0.75,
            );
            let _ = cr.stroke();
            cr.arc(12.0, 12.0, 10.0, -0.85, 0.85);
            let _ = cr.stroke();
            cr.arc(
                12.0,
                12.0,
                10.0,
                std::f64::consts::PI - 0.85,
                std::f64::consts::PI + 0.85,
            );
            let _ = cr.stroke();
        }
        LucideIcon::Laptop => {
            draw_rounded_rect(cr, 4.0, 4.0, 16.0, 12.0, 2.0);
            cr.move_to(2.0, 20.0);
            cr.line_to(22.0, 20.0);
            let _ = cr.stroke();
        }
        LucideIcon::Smartphone => {
            draw_rounded_rect(cr, 5.0, 2.0, 14.0, 20.0, 2.0);
            cr.move_to(12.0, 18.0);
            cr.line_to(12.01, 18.0);
            let _ = cr.stroke();
        }
        LucideIcon::ChevronRight => {
            cr.move_to(9.0, 18.0);
            cr.line_to(15.0, 12.0);
            cr.line_to(9.0, 6.0);
            let _ = cr.stroke();
        }
        LucideIcon::ChevronDown => {
            cr.move_to(6.0, 9.0);
            cr.line_to(12.0, 15.0);
            cr.line_to(18.0, 9.0);
            let _ = cr.stroke();
        }
        LucideIcon::ChevronUp => {
            cr.move_to(18.0, 15.0);
            cr.line_to(12.0, 9.0);
            cr.line_to(6.0, 15.0);
            let _ = cr.stroke();
        }
        LucideIcon::FileText => {
            cr.move_to(14.0, 2.0);
            cr.line_to(6.0, 2.0);
            cr.arc(
                6.0,
                4.0,
                2.0,
                -std::f64::consts::FRAC_PI_2,
                std::f64::consts::PI,
            );
            cr.line_to(4.0, 20.0);
            cr.arc(
                6.0,
                20.0,
                2.0,
                std::f64::consts::PI,
                std::f64::consts::FRAC_PI_2,
            );
            cr.line_to(18.0, 22.0);
            cr.arc(18.0, 20.0, 2.0, std::f64::consts::FRAC_PI_2, 0.0);
            cr.line_to(20.0, 8.0);
            cr.close_path();
            let _ = cr.stroke();
            cr.move_to(14.0, 2.0);
            cr.line_to(14.0, 8.0);
            cr.line_to(20.0, 8.0);
            let _ = cr.stroke();
            cr.move_to(8.0, 13.0);
            cr.line_to(16.0, 13.0);
            let _ = cr.stroke();
            cr.move_to(8.0, 17.0);
            cr.line_to(16.0, 17.0);
            let _ = cr.stroke();
        }
        LucideIcon::Folder => {
            cr.move_to(4.0, 4.0);
            cr.line_to(9.0, 4.0);
            cr.line_to(11.0, 7.0);
            cr.line_to(20.0, 7.0);
            cr.arc(20.0, 9.0, 2.0, -std::f64::consts::FRAC_PI_2, 0.0);
            cr.line_to(22.0, 18.0);
            cr.arc(20.0, 18.0, 2.0, 0.0, std::f64::consts::FRAC_PI_2);
            cr.line_to(4.0, 20.0);
            cr.arc(
                4.0,
                18.0,
                2.0,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::PI,
            );
            cr.line_to(2.0, 6.0);
            cr.arc(
                4.0,
                6.0,
                2.0,
                std::f64::consts::PI,
                -std::f64::consts::FRAC_PI_2,
            );
            cr.close_path();
            let _ = cr.stroke();
        }
        LucideIcon::Clipboard => {
            draw_rounded_rect(cr, 8.0, 2.0, 8.0, 4.0, 1.0);
            cr.move_to(16.0, 4.0);
            cr.line_to(18.0, 4.0);
            cr.arc(18.0, 6.0, 2.0, -std::f64::consts::FRAC_PI_2, 0.0);
            cr.line_to(20.0, 20.0);
            cr.arc(18.0, 20.0, 2.0, 0.0, std::f64::consts::FRAC_PI_2);
            cr.line_to(6.0, 22.0);
            cr.arc(
                6.0,
                20.0,
                2.0,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::PI,
            );
            cr.line_to(4.0, 6.0);
            cr.arc(
                6.0,
                6.0,
                2.0,
                std::f64::consts::PI,
                -std::f64::consts::FRAC_PI_2,
            );
            cr.line_to(8.0, 4.0);
            let _ = cr.stroke();
        }
        LucideIcon::Plus => {
            cr.move_to(12.0, 5.0);
            cr.line_to(12.0, 19.0);
            let _ = cr.stroke();
            cr.move_to(5.0, 12.0);
            cr.line_to(19.0, 12.0);
            let _ = cr.stroke();
        }
        LucideIcon::X => {
            cr.move_to(18.0, 6.0);
            cr.line_to(6.0, 18.0);
            let _ = cr.stroke();
            cr.move_to(6.0, 6.0);
            cr.line_to(18.0, 18.0);
            let _ = cr.stroke();
        }
        LucideIcon::RefreshCw => {
            cr.arc(12.0, 12.0, 8.0, -0.5, std::f64::consts::PI - 0.5);
            let _ = cr.stroke();
            cr.arc(
                12.0,
                12.0,
                8.0,
                std::f64::consts::PI - 0.5,
                std::f64::consts::TAU - 0.5,
            );
            let _ = cr.stroke();
            cr.move_to(17.0, 3.0);
            cr.line_to(20.0, 7.0);
            cr.line_to(16.0, 8.0);
            let _ = cr.stroke();
            cr.move_to(7.0, 21.0);
            cr.line_to(4.0, 17.0);
            cr.line_to(8.0, 16.0);
            let _ = cr.stroke();
        }
        LucideIcon::Trash2 => {
            cr.move_to(3.0, 6.0);
            cr.line_to(21.0, 6.0);
            let _ = cr.stroke();
            cr.move_to(8.0, 6.0);
            cr.line_to(8.0, 4.0);
            cr.line_to(16.0, 4.0);
            cr.line_to(16.0, 6.0);
            let _ = cr.stroke();
            cr.move_to(19.0, 6.0);
            cr.line_to(18.0, 20.0);
            cr.line_to(6.0, 20.0);
            cr.line_to(5.0, 6.0);
            let _ = cr.stroke();
            cr.move_to(10.0, 11.0);
            cr.line_to(10.0, 16.0);
            let _ = cr.stroke();
            cr.move_to(14.0, 11.0);
            cr.line_to(14.0, 16.0);
            let _ = cr.stroke();
        }
    }
}
