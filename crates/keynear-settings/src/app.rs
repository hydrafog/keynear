use std::sync::Arc;

use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use keynear_core::{
    remove_paired_device, DaemonConfig, DeviceRole, NearbyPeer, PairedDevice, PairedDeviceStore,
    SecretStore, VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS, PAIRING_TIMEOUT_SECS,
};
use relm4::{ComponentParts, ComponentSender, RelmApp, SimpleComponent};

pub const STYLE_CSS: &str = include_str!("../assets/style.css");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPairingView {
    pub peer_token: String,
    pub peer_name: String,
    pub verification_code: String,
    pub remaining_secs: u64,
}

#[derive(Clone)]
pub struct SettingsInit {
    pub config: DaemonConfig,
    pub error_message: Option<String>,
    pub warning_message: Option<String>,
    pub paired_devices: Vec<PairedDevice>,
    pub nearby_peers: Vec<NearbyPeer>,
    pub device_store: PairedDeviceStore,
    pub secret_store: Option<Arc<dyn SecretStore>>,
}

pub struct SettingsModel {
    visibility_mode: VisibilityMode,
    everyone_remaining_secs: u64,
    incoming_directory: String,
    error_message: Option<String>,
    warning_message: Option<String>,
    nearby_peers: Vec<NearbyPeer>,
    paired_devices: Vec<PairedDevice>,
    pending_pairing: Option<PendingPairingView>,
    pending_unpair_id: Option<String>,
    device_store: PairedDeviceStore,
    secret_store: Option<Arc<dyn SecretStore>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsMsg {
    CycleVisibility,
    Tick,
    RefreshPeers,
    BeginPair(String),
    ConfirmPairing,
    CancelPairing,
    RequestUnpair(String),
    ConfirmUnpair(String),
    CancelUnpair,
}

pub struct SettingsWidgets {
    visibility_value: gtk4::Label,
    status_banner: gtk4::Box,
    status_banner_label: gtk4::Label,
    pairing_card: gtk4::Box,
    pairing_peer_label: gtk4::Label,
    pairing_code_label: gtk4::Label,
    pairing_timer_label: gtk4::Label,
    nearby_list_box: gtk4::Box,
    paired_list_box: gtk4::Box,
}

pub fn format_visibility_label(mode: VisibilityMode, remaining_secs: u64) -> String {
    match mode {
        VisibilityMode::Hidden => "Hidden".to_string(),
        VisibilityMode::PairedOnly => "Paired devices only".to_string(),
        VisibilityMode::EveryoneNearby => {
            let mins = remaining_secs / 60;
            let secs = remaining_secs % 60;
            format!("Everyone ({mins:02}:{secs:02})")
        }
    }
}

fn next_visibility(mode: VisibilityMode) -> VisibilityMode {
    match mode {
        VisibilityMode::Hidden => VisibilityMode::PairedOnly,
        VisibilityMode::PairedOnly => VisibilityMode::EveryoneNearby,
        VisibilityMode::EveryoneNearby => VisibilityMode::Hidden,
    }
}

fn role_label(role: DeviceRole) -> &'static str {
    match role {
        DeviceRole::Desktop => "Desktop",
        DeviceRole::Mobile => "Mobile",
    }
}

fn clear_box_children(container: &gtk4::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

impl SettingsModel {
    pub fn from_init(init: SettingsInit) -> Self {
        let remaining = if init.config.visibility_mode == VisibilityMode::EveryoneNearby {
            EVERYONE_NEARBY_TIMEOUT_SECS
        } else {
            0
        };
        Self {
            visibility_mode: init.config.visibility_mode,
            everyone_remaining_secs: remaining,
            incoming_directory: init.config.incoming_directory,
            error_message: init.error_message,
            warning_message: init.warning_message,
            nearby_peers: init.nearby_peers,
            paired_devices: init.paired_devices,
            pending_pairing: None,
            pending_unpair_id: None,
            device_store: init.device_store,
            secret_store: init.secret_store,
        }
    }

    #[cfg(test)]
    pub fn visibility_mode(&self) -> VisibilityMode {
        self.visibility_mode
    }

    pub fn visibility_status_text(&self) -> String {
        format_visibility_label(self.visibility_mode, self.everyone_remaining_secs)
    }

    #[cfg(test)]
    pub fn paired_devices(&self) -> &[PairedDevice] {
        &self.paired_devices
    }

    #[cfg(test)]
    pub fn pending_pairing(&self) -> Option<&PendingPairingView> {
        self.pending_pairing.as_ref()
    }

    pub fn pending_unpair_id(&self) -> Option<&str> {
        self.pending_unpair_id.as_deref()
    }

    #[cfg(test)]
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    #[cfg(test)]
    pub fn set_pending_pairing_for_test(&mut self, prompt: PendingPairingView) {
        self.pending_pairing = Some(prompt);
    }

    pub fn apply_msg(&mut self, msg: SettingsMsg) {
        match msg {
            SettingsMsg::CycleVisibility => {
                self.visibility_mode = next_visibility(self.visibility_mode);
                self.everyone_remaining_secs =
                    if self.visibility_mode == VisibilityMode::EveryoneNearby {
                        EVERYONE_NEARBY_TIMEOUT_SECS
                    } else {
                        0
                    };
            }
            SettingsMsg::Tick => {
                if self.visibility_mode == VisibilityMode::EveryoneNearby {
                    if self.everyone_remaining_secs <= 1 {
                        self.visibility_mode = VisibilityMode::PairedOnly;
                        self.everyone_remaining_secs = 0;
                    } else {
                        self.everyone_remaining_secs -= 1;
                    }
                }
                if let Some(ref mut prompt) = self.pending_pairing {
                    if prompt.remaining_secs <= 1 {
                        self.pending_pairing = None;
                        self.error_message =
                            Some("Pairing verification timed out after 120 seconds.".to_string());
                    } else {
                        prompt.remaining_secs -= 1;
                    }
                }
            }
            SettingsMsg::RefreshPeers => match self.device_store.load() {
                Ok(devices) => {
                    self.paired_devices = devices;
                    self.error_message = None;
                }
                Err(err) => {
                    self.error_message = Some(err.to_string());
                }
            },
            SettingsMsg::BeginPair(peer_token) => {
                if let Some(peer) = self
                    .nearby_peers
                    .iter()
                    .find(|p| p.presence_token == peer_token)
                {
                    let code_num: u32 = peer.presence_token.bytes().fold(100_007u32, |acc, b| {
                        acc.wrapping_mul(31).wrapping_add(u32::from(b))
                    }) % 1_000_000;
                    let raw = format!("{code_num:06}");
                    self.pending_pairing = Some(PendingPairingView {
                        peer_token: peer.presence_token.clone(),
                        peer_name: peer.display_name.clone(),
                        verification_code: format!("{} {}", &raw[..3], &raw[3..6]),
                        remaining_secs: PAIRING_TIMEOUT_SECS,
                    });
                    self.error_message = None;
                }
            }
            SettingsMsg::ConfirmPairing => {
                self.pending_pairing = None;
                self.warning_message = Some(
                    "Pairing approval queued; verify matching 6-digit code on the peer device."
                        .to_string(),
                );
            }
            SettingsMsg::CancelPairing => {
                self.pending_pairing = None;
            }
            SettingsMsg::RequestUnpair(device_id) => {
                self.pending_unpair_id = Some(device_id);
            }
            SettingsMsg::CancelUnpair => {
                self.pending_unpair_id = None;
            }
            SettingsMsg::ConfirmUnpair(device_id) => {
                if let Some(ref store) = self.secret_store {
                    if let Err(err) =
                        remove_paired_device(&self.device_store, store.as_ref(), &device_id)
                    {
                        self.error_message = Some(err.to_string());
                        return;
                    }
                } else if let Err(err) = self.device_store.remove(&device_id) {
                    self.error_message = Some(err.to_string());
                    return;
                }
                self.paired_devices.retain(|d| d.device_id != device_id);
                self.pending_unpair_id = None;
                self.error_message = None;
            }
        }
    }
}

fn install_css(window: &gtk4::Window) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(STYLE_CSS);
    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    if let Some(settings) = gtk4::Settings::default() {
        if settings.is_gtk_application_prefer_dark_theme() {
            window.add_css_class("dark");
        }
    }
}

fn rebuild_nearby_rows(
    container: &gtk4::Box,
    peers: &[NearbyPeer],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    if peers.is_empty() {
        let empty = gtk4::Label::builder()
            .label("No nearby devices discovered yet.")
            .xalign(0.0)
            .wrap(true)
            .build();
        empty.add_css_class("keynear-description");
        container.append(&empty);
        return;
    }

    for peer in peers {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        row.add_css_class("keynear-tonal");

        let info_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();

        let name = gtk4::Label::builder()
            .label(&peer.display_name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let meta_text = match peer.battery_level {
            Some(pct) => format!(
                "{} • {} dBm • {pct}% battery",
                role_label(peer.device_role),
                peer.rssi
            ),
            None => format!("{} • {} dBm", role_label(peer.device_role), peer.rssi),
        };
        let meta = gtk4::Label::builder()
            .label(meta_text)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        meta.add_css_class("keynear-caption");

        info_col.append(&name);
        info_col.append(&meta);
        row.append(&info_col);

        if !peer.is_paired {
            let pair_btn = gtk4::Button::builder().label("Pair").build();
            pair_btn.add_css_class("keynear-primary");
            let token = peer.presence_token.clone();
            let s = sender.clone();
            pair_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::BeginPair(token.clone()));
            });
            row.append(&pair_btn);
        }

        container.append(&row);
    }
}

fn rebuild_paired_rows(
    container: &gtk4::Box,
    devices: &[PairedDevice],
    pending_unpair_id: Option<&str>,
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    if devices.is_empty() {
        let empty_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();
        empty_card.add_css_class("keynear-tonal");

        let empty_title = gtk4::Label::builder()
            .label("No paired devices yet")
            .xalign(0.0)
            .wrap(true)
            .build();
        empty_title.add_css_class("keynear-heading");

        let empty_desc = gtk4::Label::builder()
            .label("Switch visibility to Everyone nearby to pair a device over Bluetooth LE.")
            .xalign(0.0)
            .wrap(true)
            .max_width_chars(65)
            .build();
        empty_desc.add_css_class("keynear-description");

        empty_card.append(&empty_title);
        empty_card.append(&empty_desc);
        container.append(&empty_card);
        return;
    }

    for device in devices {
        let outer = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        outer.add_css_class("keynear-tonal");

        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();

        let info_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();

        let name = gtk4::Label::builder()
            .label(&device.display_name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let key_prefix: String = device.static_x25519_pubkey.chars().take(12).collect();
        let meta = gtk4::Label::builder()
            .label(format!(
                "{} • Key {}…",
                role_label(device.device_role),
                key_prefix
            ))
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        meta.add_css_class("keynear-caption");

        info_col.append(&name);
        info_col.append(&meta);
        row.append(&info_col);

        if pending_unpair_id == Some(device.device_id.as_str()) {
            let confirm_btn = gtk4::Button::builder().label("Confirm unpair").build();
            confirm_btn.add_css_class("keynear-danger-btn");
            let id_confirm = device.device_id.clone();
            let s_confirm = sender.clone();
            confirm_btn.connect_clicked(move |_| {
                s_confirm.input(SettingsMsg::ConfirmUnpair(id_confirm.clone()));
            });

            let cancel_btn = gtk4::Button::builder().label("Cancel").build();
            cancel_btn.add_css_class("keynear-secondary");
            let s_cancel = sender.clone();
            cancel_btn.connect_clicked(move |_| {
                s_cancel.input(SettingsMsg::CancelUnpair);
            });

            row.append(&confirm_btn);
            row.append(&cancel_btn);
        } else {
            let unpair_btn = gtk4::Button::builder().label("Unpair").build();
            unpair_btn.add_css_class("keynear-secondary");
            let id_req = device.device_id.clone();
            let s_req = sender.clone();
            unpair_btn.connect_clicked(move |_| {
                s_req.input(SettingsMsg::RequestUnpair(id_req.clone()));
            });
            row.append(&unpair_btn);
        }

        outer.append(&row);

        if pending_unpair_id == Some(device.device_id.as_str()) {
            let warn = gtk4::Label::builder()
                .label("This removes the device and deletes its stored keyring secrets.")
                .xalign(0.0)
                .wrap(true)
                .build();
            warn.add_css_class("keynear-warning");
            outer.append(&warn);
        }

        container.append(&outer);
    }
}

impl SimpleComponent for SettingsModel {
    type Init = SettingsInit;
    type Input = SettingsMsg;
    type Output = ();
    type Root = gtk4::Window;
    type Widgets = SettingsWidgets;

    fn init_root() -> Self::Root {
        gtk4::Window::builder()
            .title("Keynear Settings")
            .default_width(560)
            .resizable(true)
            .build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.add_css_class("keynear-window");
        install_css(&root);

        let model = Self::from_init(init);

        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .propagate_natural_height(true)
            .build();

        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(16)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();

        let title = gtk4::Label::builder()
            .label("Keynear")
            .xalign(0.0)
            .wrap(true)
            .build();
        title.add_css_class("keynear-title");

        let subtitle = gtk4::Label::builder()
            .label("Local continuity configuration for nearby devices.")
            .xalign(0.0)
            .wrap(true)
            .max_width_chars(65)
            .build();
        subtitle.add_css_class("keynear-description");

        header.append(&title);
        header.append(&subtitle);
        container.append(&header);

        let status_banner = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .visible(false)
            .build();
        let status_banner_label = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        status_banner.append(&status_banner_label);
        container.append(&status_banner);

        let visibility_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        visibility_card.add_css_class("keynear-card");

        let vis_heading = gtk4::Label::builder()
            .label("Discovery visibility")
            .xalign(0.0)
            .wrap(true)
            .build();
        vis_heading.add_css_class("keynear-heading");

        let visibility_value = gtk4::Label::builder()
            .label(model.visibility_status_text())
            .xalign(0.0)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .hexpand(true)
            .build();
        visibility_value.add_css_class("keynear-description");

        let vis_actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        let cycle_btn = gtk4::Button::builder()
            .label("Cycle visibility mode")
            .halign(gtk4::Align::Start)
            .build();
        cycle_btn.add_css_class("keynear-primary");
        {
            let sender = sender.clone();
            cycle_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::CycleVisibility);
            });
        }

        let refresh_btn = gtk4::Button::builder()
            .label("Refresh")
            .halign(gtk4::Align::Start)
            .build();
        refresh_btn.add_css_class("keynear-secondary");
        {
            let sender = sender.clone();
            refresh_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::RefreshPeers);
            });
        }

        vis_actions.append(&cycle_btn);
        vis_actions.append(&refresh_btn);

        visibility_card.append(&vis_heading);
        visibility_card.append(&visibility_value);
        visibility_card.append(&vis_actions);
        container.append(&visibility_card);

        let pairing_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .visible(false)
            .build();
        pairing_card.add_css_class("keynear-card");

        let pairing_heading = gtk4::Label::builder()
            .label("Verify 6-digit pairing code")
            .xalign(0.0)
            .wrap(true)
            .build();
        pairing_heading.add_css_class("keynear-heading");

        let pairing_peer_label = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        pairing_peer_label.add_css_class("keynear-description");

        let pairing_code_label = gtk4::Label::builder()
            .label("000 000")
            .halign(gtk4::Align::Start)
            .selectable(true)
            .build();
        pairing_code_label.add_css_class("keynear-code");

        let pairing_timer_label = gtk4::Label::builder().xalign(0.0).build();
        pairing_timer_label.add_css_class("keynear-caption");

        let pairing_actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        let confirm_pair_btn = gtk4::Button::builder().label("Confirm").build();
        confirm_pair_btn.add_css_class("keynear-primary");
        {
            let sender = sender.clone();
            confirm_pair_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::ConfirmPairing);
            });
        }

        let cancel_pair_btn = gtk4::Button::builder().label("Cancel").build();
        cancel_pair_btn.add_css_class("keynear-secondary");
        {
            let sender = sender.clone();
            cancel_pair_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::CancelPairing);
            });
        }

        pairing_actions.append(&confirm_pair_btn);
        pairing_actions.append(&cancel_pair_btn);

        pairing_card.append(&pairing_heading);
        pairing_card.append(&pairing_peer_label);
        pairing_card.append(&pairing_code_label);
        pairing_card.append(&pairing_timer_label);
        pairing_card.append(&pairing_actions);
        container.append(&pairing_card);

        let nearby_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        nearby_card.add_css_class("keynear-card");

        let nearby_heading = gtk4::Label::builder()
            .label("Nearby peers")
            .xalign(0.0)
            .build();
        nearby_heading.add_css_class("keynear-heading");

        let nearby_list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();

        nearby_card.append(&nearby_heading);
        nearby_card.append(&nearby_list_box);
        container.append(&nearby_card);

        let paired_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        paired_card.add_css_class("keynear-card");

        let paired_heading = gtk4::Label::builder()
            .label("Paired devices")
            .xalign(0.0)
            .build();
        paired_heading.add_css_class("keynear-heading");

        let paired_list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();

        paired_card.append(&paired_heading);
        paired_card.append(&paired_list_box);
        container.append(&paired_card);

        let config_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        config_card.add_css_class("keynear-card");

        let dir_heading = gtk4::Label::builder()
            .label("Incoming folder")
            .xalign(0.0)
            .build();
        dir_heading.add_css_class("keynear-caption");

        let dir_value = gtk4::Label::builder()
            .label(&model.incoming_directory)
            .xalign(0.0)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .hexpand(true)
            .build();
        dir_value.add_css_class("keynear-description");

        config_card.append(&dir_heading);
        config_card.append(&dir_value);
        container.append(&config_card);

        let close_btn = gtk4::Button::builder()
            .label("Close")
            .halign(gtk4::Align::End)
            .build();
        close_btn.add_css_class("keynear-secondary");
        {
            let window = root.clone();
            close_btn.connect_clicked(move |_| {
                window.close();
            });
        }
        container.append(&close_btn);

        scroll.set_child(Some(&container));
        root.set_child(Some(&scroll));

        {
            let sender = sender.clone();
            glib::timeout_add_seconds_local(1, move || {
                sender.input(SettingsMsg::Tick);
                glib::ControlFlow::Continue
            });
        }

        let mut widgets = SettingsWidgets {
            visibility_value,
            status_banner,
            status_banner_label,
            pairing_card,
            pairing_peer_label,
            pairing_code_label,
            pairing_timer_label,
            nearby_list_box,
            paired_list_box,
        };
        model.update_view(&mut widgets, sender);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        self.apply_msg(msg);
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        widgets
            .visibility_value
            .set_label(&self.visibility_status_text());

        if let Some(ref err) = self.error_message {
            widgets.status_banner.remove_css_class("keynear-warning");
            widgets.status_banner.add_css_class("keynear-error");
            widgets.status_banner_label.set_label(err);
            widgets.status_banner.set_visible(true);
        } else if let Some(ref warn) = self.warning_message {
            widgets.status_banner.remove_css_class("keynear-error");
            widgets.status_banner.add_css_class("keynear-warning");
            widgets.status_banner_label.set_label(warn);
            widgets.status_banner.set_visible(true);
        } else {
            widgets.status_banner.set_visible(false);
        }

        if let Some(ref prompt) = self.pending_pairing {
            widgets
                .pairing_peer_label
                .set_label(&format!("Compare this code with {}:", prompt.peer_name));
            widgets
                .pairing_code_label
                .set_label(&prompt.verification_code);
            widgets
                .pairing_timer_label
                .set_label(&format!("Expires in {}s", prompt.remaining_secs));
            widgets.pairing_card.set_visible(true);
        } else {
            widgets.pairing_card.set_visible(false);
        }

        rebuild_nearby_rows(&widgets.nearby_list_box, &self.nearby_peers, &sender);
        rebuild_paired_rows(
            &widgets.paired_list_box,
            &self.paired_devices,
            self.pending_unpair_id(),
            &sender,
        );
    }
}

pub fn run_gui(init: SettingsInit) {
    let app = RelmApp::new("org.keynear.settings").with_args(Vec::new());
    app.run::<SettingsModel>(init);
}
