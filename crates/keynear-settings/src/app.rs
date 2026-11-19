use gtk4::gdk::Display;
use gtk4::prelude::*;
use keynear_protocol::VisibilityMode;
use relm4::{ComponentParts, ComponentSender, RelmApp, SimpleComponent};

use crate::portal::{dispatch_send_for_device, spawn_portal_picker};
use crate::state::{format_bytes, format_countdown_short, ActiveTab, SubScreen};
use crate::views::components::sync_switch;
use crate::views::device_detail::{build_device_detail, update_device_detail, DeviceDetailParts};
use crate::views::receive::{
    build_receive_tab, rebuild_radar_pins, rebuild_sheet_rows, rebuild_transfers_rows,
    ReceiveTabParts,
};
use crate::views::send::{
    build_send_tab, rebuild_send_queue_rows, rebuild_send_targets, SendTabParts,
};
use crate::views::settings::{build_settings_tab, rebuild_mime_chips, SettingsTabParts};
use crate::views::shell::{
    build_bottom_nav, build_snackbar, build_status_banner, build_top_bar, install_css,
    BottomNavParts, SnackbarParts, StatusBannerParts, TopBarParts,
};

pub use crate::portal::file_uri_to_path;
pub use crate::state::{PendingPairingView, SettingsInit, SettingsModel, SettingsMsg};

pub const STYLE_CSS: &str = include_str!("../assets/style.css");

pub struct SettingsWidgets {
    main_stack: gtk4::Stack,
    topbar: TopBarParts,
    banner: StatusBannerParts,
    receive: ReceiveTabParts,
    send: SendTabParts,
    settings: SettingsTabParts,
    detail: DeviceDetailParts,
    snackbar: SnackbarParts,
    bottom_nav: BottomNavParts,
}

impl SimpleComponent for SettingsModel {
    type Init = SettingsInit;
    type Input = SettingsMsg;
    type Output = ();
    type Root = gtk4::Window;
    type Widgets = SettingsWidgets;

    fn init_root() -> Self::Root {
        let window = gtk4::Window::builder()
            .title("Keynear")
            .default_width(440)
            .default_height(840)
            .build();
        window.add_css_class("keynear-window");
        window
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = SettingsModel::from_init(init);
        model.root_window = Some(root.clone());
        if model.ipc_socket_path.is_some() {
            crate::portal::refresh_from_daemon(&mut model, false);
        }
        install_css(&root, model.dark_theme);

        {
            let tick_sender = sender.clone();
            gtk4::glib::timeout_add_seconds_local(1, move || {
                tick_sender.input(SettingsMsg::Tick);
                gtk4::glib::ControlFlow::Continue
            });
        }

        let shell_overlay = gtk4::Overlay::new();
        let outer_vbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(0)
            .vexpand(true)
            .build();

        let topbar = build_top_bar(&sender);
        let banner = build_status_banner(&sender);
        let receive = build_receive_tab(&sender);
        let send = build_send_tab(&sender);
        let settings = build_settings_tab(&sender);
        let detail = build_device_detail(&sender);
        let bottom_nav = build_bottom_nav(&sender);
        let snackbar = build_snackbar();

        let main_stack = gtk4::Stack::builder()
            .vexpand(true)
            .hexpand(true)
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .transition_duration(180)
            .build();
        main_stack.add_named(&receive.root, Some("receive"));
        main_stack.add_named(&send.root, Some("send"));
        main_stack.add_named(&settings.root, Some("settings"));
        main_stack.add_named(&detail.root, Some("deviceDetail"));

        outer_vbox.append(&topbar.root);
        outer_vbox.append(&banner.root);
        outer_vbox.append(&main_stack);
        outer_vbox.append(&bottom_nav.root);

        shell_overlay.set_child(Some(&outer_vbox));
        shell_overlay.add_overlay(&snackbar.root);
        root.set_child(Some(&shell_overlay));

        let mut widgets = SettingsWidgets {
            main_stack,
            topbar,
            banner,
            receive,
            send,
            settings,
            detail,
            snackbar,
            bottom_nav,
        };
        model.update_view(&mut widgets, sender);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SettingsMsg::PickSendFiles => {
                spawn_portal_picker(
                    self.root_window.as_ref(),
                    &sender,
                    "Select files to send via Keynear",
                    true,
                    false,
                    SettingsMsg::StageSendPaths,
                );
            }
            SettingsMsg::PickSendFolder => {
                spawn_portal_picker(
                    self.root_window.as_ref(),
                    &sender,
                    "Select folder to send via Keynear",
                    false,
                    true,
                    SettingsMsg::StageSendPaths,
                );
            }
            SettingsMsg::PickIncomingDirectory => {
                spawn_portal_picker(
                    self.root_window.as_ref(),
                    &sender,
                    "Select Keynear incoming folder",
                    false,
                    true,
                    |paths| {
                        let first = paths
                            .into_iter()
                            .next()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        SettingsMsg::SetIncomingDirectory(first)
                    },
                );
            }
            SettingsMsg::StageClipboardForSend => {
                if let Some(display) = Display::default() {
                    let s = sender.clone();
                    display
                        .clipboard()
                        .read_text_async(gtk4::gio::Cancellable::NONE, move |res| {
                            let text = res
                                .ok()
                                .flatten()
                                .map(|g| g.to_string())
                                .unwrap_or_default();
                            if text.trim().is_empty() {
                                s.input(SettingsMsg::PortalError(
                                    "Clipboard does not contain text to stage.".to_string(),
                                ));
                            } else {
                                s.input(SettingsMsg::StageClipboardText(text));
                            }
                        });
                } else {
                    self.apply_msg(SettingsMsg::StageClipboardForSend);
                }
            }
            SettingsMsg::SendQueueToPeer(device_id) => {
                dispatch_send_for_device(self, device_id, &sender);
            }
            other => self.apply_msg(other),
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        let title_str = match self.sub_screen {
            Some(SubScreen::DeviceDetail(_)) => self
                .selected_detail_device()
                .map(|d| d.display_name.as_str())
                .unwrap_or("Device details"),
            None => match self.active_tab {
                ActiveTab::Receive => "Keynear",
                ActiveTab::Send => "Send files",
                ActiveTab::Settings => "Settings",
            },
        };
        widgets.topbar.title_label.set_label(title_str);
        widgets
            .topbar
            .back_btn
            .set_visible(self.sub_screen.is_some());

        let stack_page = match self.sub_screen {
            Some(SubScreen::DeviceDetail(_)) => "deviceDetail",
            None => match self.active_tab {
                ActiveTab::Receive => "receive",
                ActiveTab::Send => "send",
                ActiveTab::Settings => "settings",
            },
        };
        if widgets.main_stack.visible_child_name().as_deref() != Some(stack_page) {
            widgets.main_stack.set_visible_child_name(stack_page);
        }

        if let Some(ref err) = self.error_message {
            widgets.banner.root.remove_css_class("keynear-warning");
            widgets.banner.root.add_css_class("keynear-error");
            widgets.banner.label.set_label(err);
            widgets.banner.root.set_visible(true);
        } else if let Some(ref warn) = self.warning_message {
            widgets.banner.root.remove_css_class("keynear-error");
            widgets.banner.root.add_css_class("keynear-warning");
            widgets.banner.label.set_label(warn);
            widgets.banner.root.set_visible(true);
        } else {
            widgets.banner.root.set_visible(false);
        }

        let vis_buttons = [
            (&widgets.receive.vis_hidden_btn, VisibilityMode::Hidden),
            (&widgets.receive.vis_paired_btn, VisibilityMode::PairedOnly),
            (
                &widgets.receive.vis_everyone_btn,
                VisibilityMode::EveryoneNearby,
            ),
        ];
        for (btn, mode) in vis_buttons {
            btn.remove_css_class("kn-seg-btn-active");
            btn.remove_css_class("kn-seg-btn");
            btn.add_css_class(if self.visibility_mode == mode {
                "kn-seg-btn-active"
            } else {
                "kn-seg-btn"
            });
        }

        let radar_active = self.visibility_mode != VisibilityMode::Hidden;
        widgets
            .receive
            .radar_draw_state
            .set((radar_active, self.dark_theme));
        widgets.receive.radar_canvas.queue_draw();

        widgets
            .receive
            .self_node_circle
            .remove_css_class("kn-center-node-active");
        widgets
            .receive
            .self_node_circle
            .remove_css_class("kn-center-node-hidden");
        widgets
            .receive
            .self_node_circle
            .add_css_class(if radar_active {
                "kn-center-node-active"
            } else {
                "kn-center-node-hidden"
            });
        let _ = &widgets.receive.self_name_label;
        if self.visibility_mode == VisibilityMode::EveryoneNearby {
            widgets
                .receive
                .self_countdown_label
                .set_label(&format_countdown_short(self.everyone_remaining_secs));
            widgets.receive.self_countdown_label.set_visible(true);
        } else {
            widgets.receive.self_countdown_label.set_visible(false);
        }

        let pins = self.map_pins();
        rebuild_radar_pins(&widgets.receive.radar_pins_overlay, &pins, &sender);

        if let Some(ref prompt) = self.pending_pairing {
            widgets
                .receive
                .pairing_peer_label
                .set_label(&format!("Compare this code with {}:", prompt.peer_name));
            widgets
                .receive
                .pairing_code_label
                .set_label(&prompt.verification_code);
            widgets
                .receive
                .pairing_timer_label
                .set_label(&format!("Expires in {}s", prompt.remaining_secs));
            widgets.receive.pairing_card.set_visible(true);
        } else {
            widgets.receive.pairing_card.set_visible(false);
        }

        let show_transfers = !self.transfers.is_empty() || self.transfer_error.is_some();
        widgets.receive.transfers_card.set_visible(show_transfers);
        if let Some(err) = self.transfer_error.as_deref() {
            widgets.receive.transfers_error_label.set_label(err);
            widgets.receive.transfers_error_banner.set_visible(true);
        } else {
            widgets.receive.transfers_error_banner.set_visible(false);
        }
        rebuild_transfers_rows(
            &widgets.receive.transfers_list_box,
            &self.transfers,
            &sender,
        );

        widgets
            .receive
            .sheet_count_label
            .set_label(&format!("{} computer(s) in range", pins.len()));
        widgets
            .receive
            .sheet_toggle_label
            .set_label(if self.sheet_expanded {
                "Collapse"
            } else {
                "All devices"
            });
        let target_h = if self.sheet_expanded { 240 } else { 112 };
        widgets
            .receive
            .sheet_scroll
            .set_min_content_height(target_h);
        widgets
            .receive
            .sheet_scroll
            .set_max_content_height(target_h);
        rebuild_sheet_rows(&widgets.receive.sheet_list_box, self, &pins, &sender);

        widgets.send.text_tile.remove_css_class("kn-selection-tile");
        widgets
            .send
            .text_tile
            .remove_css_class("kn-selection-tile-active");
        widgets
            .send
            .text_tile
            .add_css_class(if self.text_composer_open {
                "kn-selection-tile-active"
            } else {
                "kn-selection-tile"
            });
        widgets
            .send
            .text_composer_box
            .set_visible(self.text_composer_open);
        if widgets.send.text_entry.text().as_str() != self.text_draft
            && !widgets.send.text_entry.has_focus()
        {
            widgets.send.text_entry.set_text(&self.text_draft);
        }

        let total_bytes: u64 = self.send_queue.iter().map(|i| i.size_bytes).sum();
        widgets.send.queue_heading.set_label(&format!(
            "{} item(s) ({})",
            self.send_queue.len(),
            format_bytes(total_bytes)
        ));
        widgets
            .send
            .queue_card
            .set_visible(!self.send_queue.is_empty());
        widgets
            .send
            .clear_btn
            .set_sensitive(!self.send_queue.is_empty() && !self.sending_in_flight);

        if let Some(ref err) = self.send_error {
            widgets.send.error_label.set_label(err);
            widgets.send.error_banner.set_visible(true);
        } else {
            widgets.send.error_banner.set_visible(false);
        }
        rebuild_send_queue_rows(&widgets.send.queue_list_box, &self.send_queue, &sender);
        rebuild_send_targets(
            &widgets.send.targets_box,
            &self.paired_devices,
            self.sending_in_flight,
            !self.send_queue.is_empty(),
            &sender,
        );

        widgets
            .settings
            .save_folder_btn
            .set_label(&self.incoming_directory);
        sync_switch(
            &widgets.settings.global_auto_accept_switch,
            self.config.auto_accept_from_paired,
        );
        sync_switch(
            &widgets.settings.clipboard_sync_switch,
            self.config.clipboard_sync_enabled,
        );
        sync_switch(
            &widgets.settings.clipboard_images_switch,
            self.config.clipboard_images_enabled,
        );
        sync_switch(
            &widgets.settings.otp_passthrough_switch,
            self.config.otp_passthrough_enabled,
        );
        widgets
            .settings
            .mime_count_label
            .set_label(&self.config.ignored_mime_types.len().to_string());
        widgets
            .settings
            .mime_chevron_down
            .set_visible(!self.mime_editor_open);
        widgets
            .settings
            .mime_chevron_up
            .set_visible(self.mime_editor_open);
        widgets
            .settings
            .mime_editor_box
            .set_visible(self.mime_editor_open);
        if widgets.settings.mime_entry.text().as_str() != self.new_mime_input
            && !widgets.settings.mime_entry.has_focus()
        {
            widgets.settings.mime_entry.set_text(&self.new_mime_input);
        }
        rebuild_mime_chips(
            &widgets.settings.mime_chips_box,
            &self.config.ignored_mime_types,
            &sender,
        );
        sync_switch(
            &widgets.settings.audio_handoff_switch,
            self.config.audio_handoff_enabled,
        );
        sync_switch(
            &widgets.settings.notify_auto_accept_switch,
            self.config.notify_auto_accept_complete,
        );
        sync_switch(&widgets.settings.dark_theme_switch, self.dark_theme);

        update_device_detail(&widgets.detail, self);

        if let Some(ref snack) = self.snackbar_message {
            widgets.snackbar.label.set_label(snack);
            widgets.snackbar.root.set_visible(true);
        } else {
            widgets.snackbar.root.set_visible(false);
        }

        let navs = [
            (
                &widgets.bottom_nav.receive_pill,
                &widgets.bottom_nav.receive_text,
                self.sub_screen.is_none() && self.active_tab == ActiveTab::Receive,
            ),
            (
                &widgets.bottom_nav.send_pill,
                &widgets.bottom_nav.send_text,
                self.sub_screen.is_none() && self.active_tab == ActiveTab::Send,
            ),
            (
                &widgets.bottom_nav.settings_pill,
                &widgets.bottom_nav.settings_text,
                self.sub_screen.is_none() && self.active_tab == ActiveTab::Settings,
            ),
        ];
        for (pill, txt, active) in navs {
            pill.remove_css_class("kn-nav-pill-active");
            pill.remove_css_class("kn-nav-pill-inactive");
            pill.add_css_class(if active {
                "kn-nav-pill-active"
            } else {
                "kn-nav-pill-inactive"
            });
            txt.remove_css_class("kn-nav-text-active");
            txt.remove_css_class("keynear-caption");
            txt.add_css_class(if active {
                "kn-nav-text-active"
            } else {
                "keynear-caption"
            });
        }
    }
}

pub fn run_gui(init: SettingsInit) {
    let app = RelmApp::new("org.keynear.settings").with_args(Vec::new());
    app.run::<SettingsModel>(init);
}
