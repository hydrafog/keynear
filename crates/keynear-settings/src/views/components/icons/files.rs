use gtk4::cairo;

use super::basic::{draw_rounded_rect, Rect};

pub(super) fn draw_file_text(cr: &cairo::Context) {
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

pub(super) fn draw_folder(cr: &cairo::Context) {
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

pub(super) fn draw_clipboard(cr: &cairo::Context) {
    draw_rounded_rect(
        cr,
        Rect {
            x: 8.0,
            y: 2.0,
            w: 8.0,
            h: 4.0,
            r: 1.0,
        },
    );
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
