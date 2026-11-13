use gtk4::gdk::Display;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use keynear_core::{DaemonConfig, VisibilityMode};
use relm4::{ComponentParts, ComponentSender, RelmApp, SimpleComponent};

pub const STYLE_CSS: &str = include_str!("../assets/style.css");

#[derive(Debug, Clone)]
pub struct SettingsInit {
    pub config: DaemonConfig,
    pub error_message: Option<String>,
}

#[derive(Debug)]
pub struct SettingsModel {
    visibility_mode: VisibilityMode,
    incoming_directory: String,
    error_message: Option<String>,
}

#[derive(Debug)]
pub enum SettingsMsg {
    CycleVisibility,
}

#[derive(Debug)]
pub struct SettingsWidgets {
    visibility_value: gtk4::Label,
}

fn visibility_label(mode: VisibilityMode) -> &'static str {
    match mode {
        VisibilityMode::Hidden => "Hidden",
        VisibilityMode::PairedOnly => "Paired devices only",
        VisibilityMode::EveryoneNearby => "Everyone nearby",
    }
}

fn next_visibility(mode: VisibilityMode) -> VisibilityMode {
    match mode {
        VisibilityMode::Hidden => VisibilityMode::PairedOnly,
        VisibilityMode::PairedOnly => VisibilityMode::EveryoneNearby,
        VisibilityMode::EveryoneNearby => VisibilityMode::Hidden,
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

impl SimpleComponent for SettingsModel {
    type Init = SettingsInit;
    type Input = SettingsMsg;
    type Output = ();
    type Root = gtk4::Window;
    type Widgets = SettingsWidgets;

    fn init_root() -> Self::Root {
        gtk4::Window::builder()
            .title("Keynear Settings")
            .default_width(520)
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

        let model = Self {
            visibility_mode: init.config.visibility_mode,
            incoming_directory: init.config.incoming_directory,
            error_message: init.error_message,
        };

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

        if let Some(ref err) = model.error_message {
            let error_box = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(4)
                .build();
            error_box.add_css_class("keynear-error");
            let err_label = gtk4::Label::builder()
                .label(err)
                .xalign(0.0)
                .wrap(true)
                .build();
            error_box.append(&err_label);
            container.append(&error_box);
        }

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
            .label(visibility_label(model.visibility_mode))
            .xalign(0.0)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .hexpand(true)
            .build();
        visibility_value.add_css_class("keynear-description");

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

        visibility_card.append(&vis_heading);
        visibility_card.append(&visibility_value);
        visibility_card.append(&cycle_btn);
        container.append(&visibility_card);

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
            .label("Device pairing activates in 0.2.0-cygnus.")
            .xalign(0.0)
            .wrap(true)
            .max_width_chars(65)
            .build();
        empty_desc.add_css_class("keynear-description");

        empty_card.append(&empty_title);
        empty_card.append(&empty_desc);
        container.append(&empty_card);

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

        root.set_child(Some(&container));

        let widgets = SettingsWidgets { visibility_value };
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            SettingsMsg::CycleVisibility => {
                self.visibility_mode = next_visibility(self.visibility_mode);
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets
            .visibility_value
            .set_label(visibility_label(self.visibility_mode));
    }
}

pub fn run_gui(init: SettingsInit) {
    let app = RelmApp::new("org.keynear.settings").with_args(Vec::new());
    app.run::<SettingsModel>(init);
}
