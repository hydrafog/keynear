use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};
use crate::views::components::{clear_box_children, lucide_icon, LucideIcon};

pub(crate) struct SettingsTabParts {
    pub(crate) root: gtk4::ScrolledWindow,
    pub(crate) save_folder_btn: gtk4::Button,
    pub(crate) global_auto_accept_switch: gtk4::Switch,
    pub(crate) clipboard_sync_switch: gtk4::Switch,
    pub(crate) clipboard_images_switch: gtk4::Switch,
    pub(crate) otp_passthrough_switch: gtk4::Switch,
    pub(crate) mime_count_label: gtk4::Label,
    pub(crate) mime_chevron_down: gtk4::DrawingArea,
    pub(crate) mime_chevron_up: gtk4::DrawingArea,
    pub(crate) mime_editor_box: gtk4::Box,
    pub(crate) mime_chips_box: gtk4::Box,
    pub(crate) mime_entry: gtk4::Entry,
    pub(crate) audio_handoff_switch: gtk4::Switch,
    pub(crate) notify_auto_accept_switch: gtk4::Switch,
    pub(crate) dark_theme_switch: gtk4::Switch,
}

fn section_heading(title: &str) -> gtk4::Label {
    let label = gtk4::Label::builder().label(title).xalign(0.0).build();
    label.add_css_class("kn-section-heading");
    label
}

fn switch_row(
    title: &str,
    description: &str,
    on_toggle: impl Fn(bool) + 'static,
) -> (gtk4::Box, gtk4::Switch) {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    row.add_css_class("kn-setting-row");

    let text_col = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let title_lbl = gtk4::Label::builder().label(title).xalign(0.0).build();
    title_lbl.add_css_class("keynear-description");
    let desc_lbl = gtk4::Label::builder()
        .label(description)
        .xalign(0.0)
        .wrap(true)
        .build();
    desc_lbl.add_css_class("keynear-caption");
    text_col.append(&title_lbl);
    text_col.append(&desc_lbl);

    let sw = gtk4::Switch::builder().valign(gtk4::Align::Center).build();
    sw.add_css_class("keynear-switch");
    sw.connect_state_set(move |_, state| {
        on_toggle(state);
        gtk4::glib::Propagation::Proceed
    });

    row.append(&text_col);
    row.append(&sw);
    (row, sw)
}

pub(crate) fn build_settings_tab(sender: &ComponentSender<SettingsModel>) -> SettingsTabParts {
    let page = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(4)
        .margin_bottom(24)
        .build();

    page.append(&section_heading("GENERAL & STORAGE"));
    let recv_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .build();
    recv_card.add_css_class("keynear-card");

    let folder_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    folder_row.add_css_class("kn-setting-row");
    let folder_text = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let folder_title = gtk4::Label::builder()
        .label("Save folder")
        .xalign(0.0)
        .build();
    folder_title.add_css_class("keynear-description");
    let folder_desc = gtk4::Label::builder()
        .label("Where received files are saved")
        .xalign(0.0)
        .build();
    folder_desc.add_css_class("keynear-caption");
    folder_text.append(&folder_title);
    folder_text.append(&folder_desc);

    let save_folder_btn = gtk4::Button::with_label("~/Downloads/Keynear");
    save_folder_btn.add_css_class("kn-tonal-action-btn");
    {
        let s = sender.clone();
        save_folder_btn.connect_clicked(move |_| s.input(SettingsMsg::PickIncomingDirectory));
    }
    folder_row.append(&folder_text);
    folder_row.append(&save_folder_btn);
    recv_card.append(&folder_row);

    let (global_aa_row, global_auto_accept_switch) = switch_row(
        "Auto-accept from paired devices",
        "Save files from paired devices without asking",
        {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::SetGlobalAutoAccept(enabled))
        },
    );
    recv_card.append(&global_aa_row);
    page.append(&recv_card);

    page.append(&section_heading("CLIPBOARD"));
    let clip_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .build();
    clip_card.add_css_class("keynear-card");

    let (clip_row, clipboard_sync_switch) =
        switch_row("Clipboard sync", "Keep clipboard text in sync", {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleClipboardSync(enabled))
        });
    let (img_row, clipboard_images_switch) = switch_row(
        "Sync clipboard images",
        "Include images in clipboard sync",
        {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleClipboardImages(enabled))
        },
    );
    let (otp_row, otp_passthrough_switch) = switch_row(
        "2FA code pass-through",
        "Pass verification codes to the computer",
        {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleOtpPassthrough(enabled))
        },
    );

    clip_card.append(&clip_row);
    clip_card.append(&img_row);
    clip_card.append(&otp_row);

    let mime_header_btn = gtk4::Button::new();
    mime_header_btn.add_css_class("kn-setting-row-btn");
    let mime_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .build();
    let mime_text = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let mime_title = gtk4::Label::builder()
        .label("Ignored clipboard formats")
        .xalign(0.0)
        .build();
    mime_title.add_css_class("keynear-description");
    let mime_sub = gtk4::Label::builder()
        .label("Formats that are never shared")
        .xalign(0.0)
        .build();
    mime_sub.add_css_class("keynear-caption");
    mime_text.append(&mime_title);
    mime_text.append(&mime_sub);

    let mime_count_label = gtk4::Label::builder().label("0").build();
    mime_count_label.add_css_class("kn-count-badge");
    let mime_chevron_down = lucide_icon(LucideIcon::ChevronDown, 20);
    let mime_chevron_up = lucide_icon(LucideIcon::ChevronUp, 20);
    mime_chevron_up.set_visible(false);

    mime_row.append(&mime_text);
    mime_row.append(&mime_count_label);
    mime_row.append(&mime_chevron_down);
    mime_row.append(&mime_chevron_up);
    mime_header_btn.set_child(Some(&mime_row));
    {
        let s = sender.clone();
        mime_header_btn.connect_clicked(move |_| s.input(SettingsMsg::ToggleMimeExpanded));
    }
    clip_card.append(&mime_header_btn);

    let mime_editor_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .visible(false)
        .build();

    let mime_chips_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();

    let mime_input_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    let mime_entry = gtk4::Entry::builder()
        .placeholder_text("application/x-secret")
        .hexpand(true)
        .build();
    mime_entry.add_css_class("kn-recessed-entry");
    {
        let s = sender.clone();
        mime_entry
            .connect_changed(move |e| s.input(SettingsMsg::UpdateMimeInput(e.text().to_string())));
    }
    {
        let s = sender.clone();
        mime_entry.connect_activate(move |_| s.input(SettingsMsg::AddIgnoredMime));
    }

    let add_mime_btn = gtk4::Button::with_label("Add");
    add_mime_btn.add_css_class("kn-loading-btn");
    {
        let s = sender.clone();
        add_mime_btn.connect_clicked(move |_| s.input(SettingsMsg::AddIgnoredMime));
    }

    mime_input_row.append(&mime_entry);
    mime_input_row.append(&add_mime_btn);
    mime_editor_box.append(&mime_chips_box);
    mime_editor_box.append(&mime_input_row);
    clip_card.append(&mime_editor_box);
    page.append(&clip_card);

    page.append(&section_heading("AUDIO & NOTIFICATIONS"));
    let audio_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .build();
    audio_card.add_css_class("keynear-card");

    let (audio_row, audio_handoff_switch) =
        switch_row("Smart audio handoff", "Move audio between devices", {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleAudioHandoff(enabled))
        });
    let (notify_row, notify_auto_accept_switch) = switch_row(
        "Completion notifications",
        "Notify when files finish saving",
        {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleNotifyAutoAccept(enabled))
        },
    );
    audio_card.append(&audio_row);
    audio_card.append(&notify_row);
    page.append(&audio_card);

    page.append(&section_heading("APPEARANCE"));
    let appear_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .build();
    appear_card.add_css_class("keynear-card");

    let (theme_row, dark_theme_switch) = switch_row("Dark theme", "Switch the app color theme", {
        let s = sender.clone();
        move |enabled| s.input(SettingsMsg::ToggleDarkTheme(enabled))
    });
    appear_card.append(&theme_row);
    page.append(&appear_card);

    let root = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&page)
        .build();

    SettingsTabParts {
        root,
        save_folder_btn,
        global_auto_accept_switch,
        clipboard_sync_switch,
        clipboard_images_switch,
        otp_passthrough_switch,
        mime_count_label,
        mime_chevron_down,
        mime_chevron_up,
        mime_editor_box,
        mime_chips_box,
        mime_entry,
        audio_handoff_switch,
        notify_auto_accept_switch,
        dark_theme_switch,
    }
}

pub(crate) fn rebuild_mime_chips(
    container: &gtk4::Box,
    mimes: &[String],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    for mime in mimes {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        row.add_css_class("kn-mime-row");

        let lbl = gtk4::Label::builder()
            .label(mime)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(EllipsizeMode::End)
            .build();
        lbl.add_css_class("keynear-caption");

        let remove_icon = lucide_icon(LucideIcon::X, 16);
        let remove_btn = gtk4::Button::builder()
            .child(&remove_icon)
            .tooltip_text("Remove MIME rule")
            .build();
        remove_btn.add_css_class("kn-error-action-btn");
        {
            let s = sender.clone();
            let val = mime.clone();
            remove_btn
                .connect_clicked(move |_| s.input(SettingsMsg::RemoveIgnoredMime(val.clone())));
        }

        row.append(&lbl);
        row.append(&remove_btn);
        container.append(&row);
    }
}
