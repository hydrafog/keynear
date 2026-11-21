use gtk4::cairo;

use super::LucideIcon;

pub(super) struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub r: f64,
}

pub(super) fn draw_rounded_rect(cr: &cairo::Context, rect: Rect) {
    let Rect { x, y, w, h, r } = rect;
    let d = std::f64::consts::FRAC_PI_2;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -d, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, d);
    cr.arc(x + r, y + h - r, r, d, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * d);
    cr.close_path();
    let _ = cr.stroke();
}

pub(super) fn draw_nav_icons(cr: &cairo::Context, icon: LucideIcon) {
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
        _ => {}
    }
}

pub(super) fn draw_send(cr: &cairo::Context) {
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

pub(super) fn draw_sliders(cr: &cairo::Context) {
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

pub(super) fn draw_devices(cr: &cairo::Context, icon: LucideIcon) {
    match icon {
        LucideIcon::Laptop => {
            draw_rounded_rect(
                cr,
                Rect {
                    x: 4.0,
                    y: 4.0,
                    w: 16.0,
                    h: 12.0,
                    r: 2.0,
                },
            );
            cr.move_to(2.0, 20.0);
            cr.line_to(22.0, 20.0);
            let _ = cr.stroke();
        }
        LucideIcon::Smartphone => {
            draw_rounded_rect(
                cr,
                Rect {
                    x: 5.0,
                    y: 2.0,
                    w: 14.0,
                    h: 20.0,
                    r: 2.0,
                },
            );
            cr.move_to(12.0, 18.0);
            cr.line_to(12.01, 18.0);
            let _ = cr.stroke();
        }
        _ => {}
    }
}

pub(super) fn draw_plus_x(cr: &cairo::Context, icon: LucideIcon) {
    match icon {
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
        _ => {}
    }
}
