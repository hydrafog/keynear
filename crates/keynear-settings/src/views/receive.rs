use std::cell::Cell;
use std::rc::Rc;

use gtk4::cairo;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use keynear_core::{TransferDirection, TransferSession, TransferState, VisibilityMode};
use relm4::ComponentSender;

use crate::state::{orbit_for, role_label, MapPin, SettingsModel, SettingsMsg};
use crate::views::clear_box_children;

pub(crate) struct ReceiveTabParts {
    pub(crate) root: gtk4::Box,
    pub(crate) vis_hidden_btn: gtk4::Button,
    pub(crate) vis_paired_btn: gtk4::Button,
    pub(crate) vis_everyone_btn: gtk4::Button,
    pub(crate) radar_draw_state: Rc<Cell<(bool, bool)>>,
    pub(crate) radar_canvas: gtk4::DrawingArea,
    pub(crate) self_node_circle: gtk4::Box,
    pub(crate) self_name_label: gtk4::Label,
    pub(crate) self_countdown_label: gtk4::Label,
    pub(crate) radar_pins_overlay: gtk4::Overlay,
    pub(crate) pairing_card: gtk4::Box,
    pub(crate) pairing_peer_label: gtk4::Label,
    pub(crate) pairing_code_label: gtk4::Label,
    pub(crate) pairing_timer_label: gtk4::Label,
    pub(crate) transfers_card: gtk4::Box,
    pub(crate) transfers_error_banner: gtk4::Box,
    pub(crate) transfers_error_label: gtk4::Label,
    pub(crate) transfers_list_box: gtk4::Box,
    pub(crate) sheet_count_label: gtk4::Label,
    pub(crate) sheet_toggle_label: gtk4::Label,
    pub(crate) sheet_scroll: gtk4::ScrolledWindow,
    pub(crate) sheet_list_box: gtk4::Box,
}

fn draw_radar_background(cr: &cairo::Context, width: f64, height: f64, active: bool, dark: bool) {
    let cx = width * 0.5;
    let cy = height * 0.5;

    let aura = cairo::RadialGradient::new(cx, cy, 4.0, cx, cy, 155.0);
    let aura_alpha = if active {
        if dark {
            0.28
        } else {
            0.22
        }
    } else {
        0.04
    };
    aura.add_color_stop_rgba(0.0, 0.004, 0.584, 0.831, aura_alpha);
    aura.add_color_stop_rgba(0.7, 0.004, 0.584, 0.831, aura_alpha * 0.25);
    aura.add_color_stop_rgba(1.0, 0.004, 0.584, 0.831, 0.0);
    let _ = cr.set_source(&aura);
    cr.arc(cx, cy, 155.0, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();

    if dark {
        cr.set_source_rgba(0.604, 0.604, 0.651, 0.16);
    } else {
        cr.set_source_rgba(0.027, 0.075, 0.169, 0.12);
    }
    cr.set_line_width(1.0);
    cr.arc(cx, cy, 80.0, 0.0, std::f64::consts::TAU);
    let _ = cr.stroke();
    cr.arc(cx, cy, 180.0, 0.0, std::f64::consts::TAU);
    let _ = cr.stroke();

    if active {
        cr.set_source_rgba(0.004, 0.584, 0.831, if dark { 0.28 } else { 0.22 });
        cr.set_line_width(1.5);
        cr.arc(cx, cy, 108.0, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();

        cr.set_source_rgba(0.004, 0.584, 0.831, if dark { 0.16 } else { 0.12 });
        cr.arc(cx, cy, 146.0, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();
    }
}

pub(crate) fn build_receive_tab(sender: &ComponentSender<SettingsModel>) -> ReceiveTabParts {
    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(0)
        .vexpand(true)
        .build();
    root.add_css_class("kn-map-ground");

    let top_controls = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_top(10)
        .build();

    let seg_well = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(4)
        .build();
    seg_well.add_css_class("kn-track-well");

    let mk_seg = |label: &str, mode: VisibilityMode| -> gtk4::Button {
        let btn = gtk4::Button::with_label(label);
        btn.add_css_class("kn-seg-btn");
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(SettingsMsg::SetVisibility(mode)));
        btn
    };

    let vis_hidden_btn = mk_seg("Hidden", VisibilityMode::Hidden);
    let vis_paired_btn = mk_seg("Paired", VisibilityMode::PairedOnly);
    let vis_everyone_btn = mk_seg("Everyone", VisibilityMode::EveryoneNearby);

    seg_well.append(&vis_hidden_btn);
    seg_well.append(&vis_paired_btn);
    seg_well.append(&vis_everyone_btn);
    top_controls.append(&seg_well);
    root.append(&top_controls);

    let radar_draw_state = Rc::new(Cell::new((true, false)));
    let radar_canvas = gtk4::DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    {
        let draw_state = radar_draw_state.clone();
        radar_canvas.set_draw_func(move |_area, cr, width, height| {
            let (active, dark) = draw_state.get();
            draw_radar_background(cr, f64::from(width), f64::from(height), active, dark);
        });
    }

    let radar_stage = gtk4::Overlay::builder().vexpand(true).hexpand(true).build();
    radar_stage.add_css_class("kn-radar-stage");
    radar_stage.set_child(Some(&radar_canvas));

    let radar_pins_overlay = gtk4::Overlay::builder().vexpand(true).hexpand(true).build();

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

    let self_icon = gtk4::Image::from_icon_name("computer-symbolic");
    self_icon.set_pixel_size(28);
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

    let host_label = gtk4::Label::builder().label("This computer").build();
    host_label.add_css_class("keynear-description");

    let self_countdown_label = gtk4::Label::builder().label("").visible(false).build();
    self_countdown_label.add_css_class("kn-countdown-text");

    self_pill.append(&host_label);
    self_pill.append(&self_countdown_label);

    center_box.append(&self_node_circle);
    center_box.append(&self_pill);

    radar_pins_overlay.set_child(Some(&center_box));
    radar_stage.add_overlay(&radar_pins_overlay);
    root.append(&radar_stage);

    let pairing_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(8)
        .visible(false)
        .build();
    pairing_card.add_css_class("keynear-card");

    let pairing_title = gtk4::Label::builder()
        .label("PAIRING VERIFICATION")
        .xalign(0.0)
        .build();
    pairing_title.add_css_class("kn-section-heading");

    let pairing_peer_label = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .wrap(true)
        .build();
    pairing_peer_label.add_css_class("keynear-description");

    let pairing_code_label = gtk4::Label::builder().label("000 000").xalign(0.5).build();
    pairing_code_label.add_css_class("keynear-code");

    let pairing_timer_label = gtk4::Label::builder().label("").xalign(0.5).build();
    pairing_timer_label.add_css_class("keynear-caption");

    let pairing_actions = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .homogeneous(true)
        .build();

    let pair_reject_btn = gtk4::Button::with_label("Decline");
    pair_reject_btn.add_css_class("kn-error-action-btn");
    {
        let s = sender.clone();
        pair_reject_btn.connect_clicked(move |_| s.input(SettingsMsg::CancelPairing));
    }

    let pair_confirm_btn = gtk4::Button::with_label("Confirm");
    pair_confirm_btn.add_css_class("kn-loading-btn");
    {
        let s = sender.clone();
        pair_confirm_btn.connect_clicked(move |_| s.input(SettingsMsg::ConfirmPairing));
    }

    pairing_actions.append(&pair_reject_btn);
    pairing_actions.append(&pair_confirm_btn);
    pairing_card.append(&pairing_title);
    pairing_card.append(&pairing_peer_label);
    pairing_card.append(&pairing_code_label);
    pairing_card.append(&pairing_timer_label);
    pairing_card.append(&pairing_actions);
    root.append(&pairing_card);

    let transfers_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(8)
        .visible(false)
        .build();
    transfers_card.add_css_class("keynear-card");

    let transfers_heading = gtk4::Label::builder()
        .label("ACTIVE TRANSFERS")
        .xalign(0.0)
        .build();
    transfers_heading.add_css_class("kn-section-heading");

    let transfers_error_banner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .visible(false)
        .build();
    transfers_error_banner.add_css_class("keynear-error");
    let transfers_error_label = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .wrap(true)
        .build();
    transfers_error_label.add_css_class("keynear-description");
    transfers_error_banner.append(&transfers_error_label);

    let transfers_list_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();

    transfers_card.append(&transfers_heading);
    transfers_card.append(&transfers_error_banner);
    transfers_card.append(&transfers_list_box);
    root.append(&transfers_card);

    let sheet_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .build();
    sheet_card.add_css_class("kn-bottom-sheet");

    let handle = gtk4::Box::builder()
        .halign(gtk4::Align::Center)
        .width_request(36)
        .height_request(4)
        .build();
    handle.add_css_class("kn-sheet-handle");
    sheet_card.append(&handle);

    let sheet_header = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();

    let sheet_titles = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let sheet_title = gtk4::Label::builder()
        .label("Devices in range")
        .xalign(0.0)
        .build();
    sheet_title.add_css_class("keynear-section-title");
    let sheet_count_label = gtk4::Label::builder()
        .label("0 devices")
        .xalign(0.0)
        .build();
    sheet_count_label.add_css_class("keynear-caption");
    sheet_titles.append(&sheet_title);
    sheet_titles.append(&sheet_count_label);

    let sheet_toggle_label = gtk4::Label::builder().label("All devices").build();
    sheet_toggle_label.add_css_class("kn-link-label");
    let sheet_toggle_btn = gtk4::Button::builder().child(&sheet_toggle_label).build();
    sheet_toggle_btn.add_css_class("kn-link-btn");
    {
        let s = sender.clone();
        sheet_toggle_btn.connect_clicked(move |_| s.input(SettingsMsg::ToggleSheetExpanded));
    }

    let refresh_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh devices and daemon state")
        .build();
    refresh_btn.add_css_class("kn-tonal-action-btn");
    {
        let s = sender.clone();
        refresh_btn.connect_clicked(move |_| s.input(SettingsMsg::RefreshPeers));
    }

    sheet_header.append(&sheet_titles);
    sheet_header.append(&sheet_toggle_btn);
    sheet_header.append(&refresh_btn);
    sheet_card.append(&sheet_header);

    let sheet_list_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();

    let sheet_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(112)
        .max_content_height(112)
        .child(&sheet_list_box)
        .build();

    sheet_card.append(&sheet_scroll);
    root.append(&sheet_card);

    ReceiveTabParts {
        root,
        vis_hidden_btn,
        vis_paired_btn,
        vis_everyone_btn,
        radar_draw_state,
        radar_canvas,
        self_node_circle,
        self_name_label: host_label,
        self_countdown_label,
        radar_pins_overlay,
        pairing_card,
        pairing_peer_label,
        pairing_code_label,
        pairing_timer_label,
        transfers_card,
        transfers_error_banner,
        transfers_error_label,
        transfers_list_box,
        sheet_count_label,
        sheet_toggle_label,
        sheet_scroll,
        sheet_list_box,
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

pub(crate) fn rebuild_radar_pins(
    overlay: &gtk4::Overlay,
    pins: &[MapPin],
    sender: &ComponentSender<SettingsModel>,
) {
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

    let count = pins.len();
    for (idx, pin) in pins.iter().enumerate() {
        let orbit = orbit_for(pin.rssi, idx, count);
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

        let icon = gtk4::Image::from_icon_name(if pin.is_paired {
            "phone-symbolic"
        } else {
            "network-wireless-symbolic"
        });
        icon.set_pixel_size(24);
        icon.set_hexpand(true);
        icon.set_vexpand(true);
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::Center);
        icon.add_css_class("kn-pin-icon");
        circle.append(&icon);

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
        overlay.add_overlay(&pin_btn);
    }
}

fn transfer_fraction(session: &TransferSession) -> f64 {
    if session.total_bytes == 0 {
        return match session.state {
            TransferState::Completed => 1.0,
            _ => 0.0,
        };
    }
    (session.transferred_bytes as f64 / session.total_bytes as f64).clamp(0.0, 1.0)
}

fn transfer_summary(session: &TransferSession) -> String {
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

pub(crate) fn rebuild_transfers_rows(
    container: &gtk4::Box,
    transfers: &[TransferSession],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    for session in transfers {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        row.add_css_class("keynear-transfer-row");

        let top = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        let lbl = gtk4::Label::builder()
            .label(transfer_summary(session))
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(EllipsizeMode::End)
            .build();
        lbl.add_css_class("keynear-description");
        top.append(&lbl);

        match session.state {
            TransferState::PendingConsent => {
                let tid = session.transfer_id.clone();
                let accept_btn = gtk4::Button::with_label("Accept");
                accept_btn.add_css_class("kn-loading-btn");
                {
                    let s = sender.clone();
                    let id = tid.clone();
                    accept_btn
                        .connect_clicked(move |_| s.input(SettingsMsg::AcceptTransfer(id.clone())));
                }
                let decline_btn = gtk4::Button::with_label("Decline");
                decline_btn.add_css_class("kn-error-action-btn");
                {
                    let s = sender.clone();
                    decline_btn.connect_clicked(move |_| {
                        s.input(SettingsMsg::DeclineTransfer(tid.clone()))
                    });
                }
                top.append(&accept_btn);
                top.append(&decline_btn);
            }
            TransferState::Active => {
                let tid = session.transfer_id.clone();
                let cancel_btn = gtk4::Button::with_label("Cancel");
                cancel_btn.add_css_class("kn-error-action-btn");
                {
                    let s = sender.clone();
                    cancel_btn.connect_clicked(move |_| {
                        s.input(SettingsMsg::CancelTransfer(tid.clone()))
                    });
                }
                top.append(&cancel_btn);
            }
            _ => {}
        }

        let bar = gtk4::ProgressBar::new();
        bar.set_fraction(transfer_fraction(session));
        bar.add_css_class("keynear-progress");

        row.append(&top);
        row.append(&bar);
        container.append(&row);
    }
}

pub(crate) fn rebuild_sheet_rows(
    container: &gtk4::Box,
    model: &SettingsModel,
    pins: &[MapPin],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);

    if pins.is_empty() {
        let empty = gtk4::Label::builder()
            .label("Discovery is hidden. Turn on Paired or Everyone nearby to browse devices on this network.")
            .xalign(0.0)
            .wrap(true)
            .build();
        empty.add_css_class("keynear-caption");
        container.append(&empty);
        return;
    }

    for pin in pins {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        row.add_css_class("kn-sheet-row");

        let badge = gtk4::Box::builder()
            .width_request(44)
            .height_request(44)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();
        badge.add_css_class(if pin.is_paired && pin.is_online {
            "kn-loading-tonal-badge"
        } else {
            "kn-surface-tonal-badge"
        });

        let icon = gtk4::Image::from_icon_name(if pin.is_paired {
            "phone-symbolic"
        } else {
            "network-wireless-symbolic"
        });
        icon.set_pixel_size(20);
        icon.set_hexpand(true);
        icon.set_vexpand(true);
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::Center);
        badge.append(&icon);

        let text_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let name = gtk4::Label::builder()
            .label(&pin.display_name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let sub_text =
            if let Some(dev) = model.paired_devices.iter().find(|d| d.device_id == pin.id) {
                format!(
                    "{} · Paired {} · {}",
                    role_label(dev.device_role),
                    dev.paired_at,
                    if pin.is_online { "Wi-Fi" } else { "Standby" }
                )
            } else {
                format!("Nearby peer · {} dBm", pin.rssi)
            };
        let sub = gtk4::Label::builder()
            .label(&sub_text)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        sub.add_css_class("keynear-caption");

        text_col.append(&name);
        text_col.append(&sub);

        let action_btn = gtk4::Button::with_label(if pin.is_paired { "Details >" } else { "Pair" });
        action_btn.add_css_class(if pin.is_paired {
            "kn-tonal-action-btn"
        } else {
            "kn-loading-btn"
        });
        {
            let s = sender.clone();
            let id = pin.id.clone();
            let is_paired = pin.is_paired;
            action_btn.connect_clicked(move |_| {
                if is_paired {
                    s.input(SettingsMsg::OpenDeviceDetail(id.clone()));
                } else {
                    s.input(SettingsMsg::BeginPair(id.clone()));
                }
            });
        }

        row.append(&badge);
        row.append(&text_col);
        row.append(&action_btn);
        container.append(&row);
    }
}
