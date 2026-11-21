use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;

pub(super) fn current_detail_id(entry: &gtk4::Entry) -> String {
    entry.widget_name().to_string()
}

pub(super) fn section_heading(title: &str) -> gtk4::Label {
    let label = gtk4::Label::builder().label(title).xalign(0.0).build();
    label.add_css_class("kn-section-heading");
    label
}

pub(super) fn meta_row(label: &str) -> (gtk4::Box, gtk4::Label) {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    let key = gtk4::Label::builder()
        .label(label)
        .xalign(0.0)
        .hexpand(true)
        .build();
    key.add_css_class("keynear-caption");
    let val = gtk4::Label::builder()
        .label("")
        .xalign(1.0)
        .ellipsize(EllipsizeMode::End)
        .build();
    val.add_css_class("keynear-caption");
    row.append(&key);
    row.append(&val);
    (row, val)
}
