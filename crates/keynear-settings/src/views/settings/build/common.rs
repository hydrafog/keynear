use gtk4::prelude::*;

pub(super) fn section_heading(title: &str) -> gtk4::Label {
    let label = gtk4::Label::builder().label(title).xalign(0.0).build();
    label.add_css_class("kn-section-heading");
    label
}

pub(super) fn switch_row(
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
