use chck_types::{Folder, MessageSummary};
use gtk::prelude::*;
use gtk4 as gtk;

use crate::util::{folder_label, format_unix, from_display};

pub fn mail_row(msg: &MessageSummary) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("mail-row");

    let grid = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    grid.set_margin_start(10);
    grid.set_margin_end(10);
    grid.set_margin_top(8);
    grid.set_margin_bottom(8);

    let stripe = gtk::Box::new(gtk::Orientation::Vertical, 0);
    stripe.set_width_request(3);
    stripe.add_css_class("account-stripe");
    grid.append(&stripe);

    let col = gtk::Box::new(gtk::Orientation::Vertical, 2);
    col.set_hexpand(true);

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let from = gtk::Label::new(Some(&from_display(&msg.from)));
    from.set_xalign(0.0);
    from.set_hexpand(true);
    from.set_ellipsize(gtk::pango::EllipsizeMode::End);
    if !msg.flags.seen {
        from.add_css_class("heading");
    }
    let date = gtk::Label::new(Some(&format_unix(msg.date_unix)));
    date.add_css_class("dim-label");
    date.set_xalign(1.0);
    top.append(&from);
    top.append(&date);

    let subject = gtk::Label::new(Some(if msg.subject.is_empty() {
        crate::i18n::t("(无主题)")
    } else {
        msg.subject.as_str()
    }));
    subject.set_xalign(0.0);
    subject.set_ellipsize(gtk::pango::EllipsizeMode::End);
    if !msg.flags.seen {
        subject.add_css_class("heading");
    }

    let snippet = gtk::Label::new(Some(&msg.snippet));
    snippet.set_xalign(0.0);
    snippet.add_css_class("dim-label");
    snippet.set_ellipsize(gtk::pango::EllipsizeMode::End);
    snippet.set_max_width_chars(48);

    col.append(&top);
    col.append(&subject);
    col.append(&snippet);
    grid.append(&col);
    row.set_child(Some(&grid));
    row
}

pub fn folder_row(folder: &Folder) -> gtk::ListBoxRow {
    let unread = if folder.unread_count > 0 { Some(folder.unread_count.to_string()) } else { None };
    nav_row(&folder_label(folder), unread.as_deref(), Some("folder-symbolic"))
}

pub fn nav_row(title: &str, badge: Option<&str>, icon: Option<&str>) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    box_.set_margin_start(8);
    box_.set_margin_end(8);
    box_.set_margin_top(6);
    box_.set_margin_bottom(6);
    if let Some(name) = icon {
        let image = gtk::Image::from_icon_name(name);
        box_.append(&image);
    }
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    box_.append(&label);
    if let Some(badge) = badge {
        let pill = gtk::Label::new(Some(badge));
        pill.add_css_class("unread-badge");
        box_.append(&pill);
    }
    row.set_child(Some(&box_));
    row
}
