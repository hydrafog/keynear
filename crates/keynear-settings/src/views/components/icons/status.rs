use gtk4::cairo;

pub(super) fn draw_eye_off(cr: &cairo::Context) {
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

pub(super) fn draw_shield_check(cr: &cairo::Context) {
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

pub(super) fn draw_radio(cr: &cairo::Context) {
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

pub(super) fn draw_inbox(cr: &cairo::Context) {
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

pub(super) fn draw_refresh_cw(cr: &cairo::Context) {
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

pub(super) fn draw_trash2(cr: &cairo::Context) {
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
