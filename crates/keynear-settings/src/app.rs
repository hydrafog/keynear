mod view;

use std::path::PathBuf;

use gtk4::gdk::Display;
use gtk4::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmApp, SimpleComponent};

use crate::portal::{dispatch_send_for_device, spawn_portal_picker, PortalPickerParams};
use crate::views::device_detail::build_device_detail;
use crate::views::device_detail::DeviceDetailParts;
use crate::views::receive::build_receive_tab;
use crate::views::receive::ReceiveTabParts;
use crate::views::send::build_send_tab;
use crate::views::send::SendTabParts;
use crate::views::settings::build_settings_tab;
use crate::views::settings::SettingsTabParts;
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

fn build_widgets(sender: &ComponentSender<SettingsModel>, root: &gtk4::Window) -> SettingsWidgets {
    let shell_overlay = gtk4::Overlay::new();
    let outer_vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(0)
        .vexpand(true)
        .build();

    let topbar = build_top_bar(sender);
    let banner = build_status_banner(sender);
    let receive = build_receive_tab(sender);
    let send = build_send_tab(sender);
    let settings = build_settings_tab(sender);
    let detail = build_device_detail(sender);
    let bottom_nav = build_bottom_nav(sender);
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

    SettingsWidgets {
        main_stack,
        topbar,
        banner,
        receive,
        send,
        settings,
        detail,
        snackbar,
        bottom_nav,
    }
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
            .icon_name("org.keynear.Keynear")
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

        let mut widgets = build_widgets(&sender, &root);
        view::update_view(&model, &mut widgets, sender);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SettingsMsg::PickSendFiles => self.pick_files(PickFilesParams {
                sender: &sender,
                title: "Select files to send via Keynear",
                multiple: true,
                directory: false,
            }),
            SettingsMsg::PickSendFolder => self.pick_files(PickFilesParams {
                sender: &sender,
                title: "Select folder to send via Keynear",
                multiple: false,
                directory: true,
            }),
            SettingsMsg::PickIncomingDirectory => self.pick_incoming_directory(&sender),
            SettingsMsg::StageClipboardForSend => self.stage_clipboard(&sender),
            SettingsMsg::SendQueueToPeer(device_id) => {
                dispatch_send_for_device(self, device_id, &sender);
            }
            other => self.apply_msg(other),
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        view::update_view(self, widgets, sender);
    }
}

pub fn run_gui(init: SettingsInit) {
    let app = RelmApp::new("org.keynear.Keynear").with_args(Vec::new());
    app.run::<SettingsModel>(init);
}

impl SettingsModel {
    fn pick_files(&self, params: PickFilesParams<'_>) {
        spawn_portal_picker(PortalPickerParams {
            window: self.root_window.as_ref(),
            sender: params.sender,
            title: params.title,
            multiple: params.multiple,
            directory: params.directory,
            on_paths: SettingsMsg::StageSendPaths,
        });
    }

    fn pick_incoming_directory(&self, sender: &ComponentSender<Self>) {
        spawn_portal_picker(PortalPickerParams {
            window: self.root_window.as_ref(),
            sender,
            title: "Select Keynear incoming folder",
            multiple: false,
            directory: true,
            on_paths: |paths: Vec<PathBuf>| {
                let first = paths
                    .into_iter()
                    .next()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                SettingsMsg::SetIncomingDirectory(first)
            },
        });
    }

    fn stage_clipboard(&mut self, sender: &ComponentSender<Self>) {
        let Some(display) = Display::default() else {
            self.apply_msg(SettingsMsg::StageClipboardForSend);
            return;
        };
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
    }
}

struct PickFilesParams<'a> {
    sender: &'a ComponentSender<SettingsModel>,
    title: &'static str,
    multiple: bool,
    directory: bool,
}
