use std::cell::Cell;
use std::rc::Rc;

use gtk4::cairo;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{orbit_for, MapPin, OrbitPlacement, SettingsModel, SettingsMsg};
use crate::views::components::{lucide_icon, LucideIcon};

pub(crate) struct RadarStageParts {
    pub(crate) stage: gtk4::Overlay,
    pub(crate) radar_draw_state: Rc<Cell<(bool, bool)>>,
    pub(crate) radar_canvas: gtk4::DrawingArea,
    pub(crate) self_node_circle: gtk4::Box,
    pub(crate) self_name_label: gtk4::Label,
    pub(crate) self_countdown_label: gtk4::Label,
    pub(crate) radar_pins_overlay: gtk4::Overlay,
}

struct RadarDrawState {
    active: bool,
    dark: bool,
}

fn draw_radar_background(cr: &cairo::Context, width: i32, height: i32, state: &RadarDrawState) {
    let cx = f64::from(width) * 0.5;
    let cy = f64::from(height) * 0.5;

    let aura = cairo::RadialGradient::new(cx, cy, 4.0, cx, cy, 155.0);
    let aura_alpha = if !state.active {
        0.04
    } else if state.dark {
        0.28
    } else {
        0.22
    };
    aura.add_color_stop_rgba(0.0, 0.004, 0.584, 0.831, aura_alpha);
    aura.add_color_stop_rgba(0.7, 0.004, 0.584, 0.831, aura_alpha * 0.25);
    aura.add_color_stop_rgba(1.0, 0.004, 0.584, 0.831, 0.0);
    let _ = cr.set_source(&aura);
    cr.arc(cx, cy, 155.0, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();

    if state.dark {
        cr.set_source_rgba(0.604, 0.604, 0.651, 0.16);
    } else {
        cr.set_source_rgba(0.027, 0.075, 0.169, 0.12);
    }
    cr.set_line_width(1.0);
    cr.arc(cx, cy, 80.0, 0.0, std::f64::consts::TAU);
    let _ = cr.stroke();
    cr.arc(cx, cy, 180.0, 0.0, std::f64::consts::TAU);
    let _ = cr.stroke();

    if state.active {
        cr.set_source_rgba(0.004, 0.584, 0.831, if state.dark { 0.28 } else { 0.22 });
        cr.set_line_width(1.5);
        cr.arc(cx, cy, 108.0, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();

        cr.set_source_rgba(0.004, 0.584, 0.831, if state.dark { 0.16 } else { 0.12 });
        cr.arc(cx, cy, 146.0, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();
    }
}

fn build_center_box() -> (gtk4::Box, gtk4::Box, gtk4::Label, gtk4::Label) {
    let center_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();

    let self_node_circle = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .width_request(64)
        .height_request(64)
        .build();
    self_node_circle.add_css_class("kn-center-node-active");

    let self_icon = lucide_icon(LucideIcon::Laptop, 28);
    self_icon.set_hexpand(true);
    self_icon.set_vexpand(true);
    self_icon.set_halign(gtk4::Align::Center);
    self_icon.set_valign(gtk4::Align::Center);
    self_node_circle.append(&self_icon);

    let self_pill = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .halign(gtk4::Align::Center)
        .build();
    self_pill.add_css_class("kn-center-pill");

    let self_name_label = gtk4::Label::builder().label("This computer").build();
    self_name_label.add_css_class("keynear-description");

    let self_countdown_label = gtk4::Label::builder().label("").visible(false).build();
    self_countdown_label.add_css_class("kn-countdown-text");

    self_pill.append(&self_name_label);
    self_pill.append(&self_countdown_label);

    center_box.append(&self_node_circle);
    center_box.append(&self_pill);

    (
        center_box,
        self_node_circle,
        self_name_label,
        self_countdown_label,
    )
}

pub(crate) fn build_radar_stage() -> RadarStageParts {
    let radar_draw_state = Rc::new(Cell::new((true, false)));
    let radar_canvas = gtk4::DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    {
        let draw_state = radar_draw_state.clone();
        radar_canvas.set_draw_func(move |_area, cr, width, height| {
            let (active, dark) = draw_state.get();
            draw_radar_background(cr, width, height, &RadarDrawState { active, dark });
        });
    }

    let stage = gtk4::Overlay::builder().vexpand(true).hexpand(true).build();
    stage.add_css_class("kn-radar-stage");
    stage.set_child(Some(&radar_canvas));

    let radar_pins_overlay = gtk4::Overlay::builder().vexpand(true).hexpand(true).build();

    let (center_box, self_node_circle, self_name_label, self_countdown_label) = build_center_box();

    radar_pins_overlay.set_child(Some(&center_box));
    stage.add_overlay(&radar_pins_overlay);

    RadarStageParts {
        stage,
        radar_draw_state,
        radar_canvas,
        self_node_circle,
        self_name_label,
        self_countdown_label,
        radar_pins_overlay,
    }
}

fn apply_overlay_offset(widget: &gtk4::Widget, dx: i32, dy: i32) {
    let px = (f64::from(dx) * 0.72).round() as i32;
    let py = (f64::from(dy) * 0.68).round() as i32;
    widget.set_halign(gtk4::Align::Center);
    widget.set_valign(gtk4::Align::Center);
    if px >= 0 {
        widget.set_margin_start(px * 2);
        widget.set_margin_end(0);
    } else {
        widget.set_margin_start(0);
        widget.set_margin_end((-px) * 2);
    }
    if py >= 0 {
        widget.set_margin_top(py * 2);
        widget.set_margin_bottom(0);
    } else {
        widget.set_margin_top(0);
        widget.set_margin_bottom((-py) * 2);
    }
}

fn clear_pin_overlays(overlay: &gtk4::Overlay) {
    let mut to_remove = Vec::new();
    let mut child = overlay.first_child();
    let main_child = overlay.child();
    while let Some(widget) = child {
        let next = widget.next_sibling();
        if main_child.as_ref() != Some(&widget) {
            to_remove.push(widget);
        }
        child = next;
    }
    for widget in to_remove {
        overlay.remove_overlay(&widget);
    }
}

fn build_pin_card(pin: &MapPin) -> gtk4::Box {
    let card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(1)
        .halign(gtk4::Align::Center)
        .build();
    card.add_css_class("kn-pin-card");

    let name_lbl = gtk4::Label::builder()
        .label(&pin.display_name)
        .ellipsize(EllipsizeMode::End)
        .max_width_chars(14)
        .build();
    name_lbl.add_css_class("keynear-description");

    let state_lbl = gtk4::Label::builder().label(&pin.state_label).build();
    state_lbl.add_css_class(if !pin.is_paired {
        "kn-pin-state-pair"
    } else if pin.is_online {
        "kn-pin-state-online"
    } else {
        "keynear-caption"
    });

    let rssi_lbl = gtk4::Label::builder()
        .label(format!("{} dBm", pin.rssi))
        .build();
    rssi_lbl.add_css_class("keynear-caption");

    card.append(&name_lbl);
    card.append(&state_lbl);
    card.append(&rssi_lbl);
    card
}

fn build_pin_button(
    pin: &MapPin,
    orbit: &OrbitPlacement,
    sender: &ComponentSender<SettingsModel>,
) -> gtk4::Button {
    let pin_btn = gtk4::Button::new();
    pin_btn.add_css_class("kn-pin-btn");

    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .halign(gtk4::Align::Center)
        .build();

    let circle = gtk4::Box::builder()
        .halign(gtk4::Align::Center)
        .width_request(56)
        .height_request(56)
        .build();
    circle.add_css_class("kn-pin-circle");

    let icon = lucide_icon(
        if pin.is_paired {
            LucideIcon::Smartphone
        } else {
            LucideIcon::Radio
        },
        24,
    );
    icon.set_hexpand(true);
    icon.set_vexpand(true);
    icon.set_halign(gtk4::Align::Center);
    icon.set_valign(gtk4::Align::Center);
    icon.add_css_class("kn-pin-icon");
    circle.append(&icon);

    let card = build_pin_card(pin);
    vbox.append(&circle);
    vbox.append(&card);
    pin_btn.set_child(Some(&vbox));

    let id = pin.id.clone();
    let is_paired = pin.is_paired;
    let s = sender.clone();
    pin_btn.connect_clicked(move |_| {
        if is_paired {
            s.input(SettingsMsg::OpenDeviceDetail(id.clone()));
        } else {
            s.input(SettingsMsg::BeginPair(id.clone()));
        }
    });

    apply_overlay_offset(pin_btn.upcast_ref(), orbit.offset_x, orbit.offset_y);
    pin_btn
}

pub(crate) fn rebuild_radar_pins(
    overlay: &gtk4::Overlay,
    pins: &[MapPin],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_pin_overlays(overlay);

    let count = pins.len();
    for (idx, pin) in pins.iter().enumerate() {
        let orbit = orbit_for(pin.rssi, idx, count);
        let pin_btn = build_pin_button(pin, &orbit, sender);
        overlay.add_overlay(&pin_btn);
    }
}
