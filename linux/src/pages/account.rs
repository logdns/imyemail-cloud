use std::sync::Arc;

use adw::prelude::*;
use chck_types::{AddAccountRequest, ConnectProbe, ProbeStatus};
use gtk4 as gtk;
use libadwaita as adw;

use crate::engine_bridge::EngineBridge;

pub fn present(
    parent: &impl IsA<gtk::Widget>,
    bridge: Arc<EngineBridge>,
    on_added: impl Fn() + 'static,
) {
    let dialog = adw::Dialog::new();
    dialog.set_title(crate::i18n::t("添加账号"));
    dialog.set_content_width(640);

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar.add_top_bar(&header);

    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::new();
    group.set_title(crate::i18n::t("连接"));
    group.set_description(Some(crate::i18n::t(
        "命中 18 家预设即自动填充 IMAP/SMTP。密码进系统钥匙串或本地 sidecar，不进邮件库。",
    )));

    let email = adw::EntryRow::builder().title(crate::i18n::t("邮箱")).build();
    let password = adw::PasswordEntryRow::builder().title(crate::i18n::t("密码 / 授权码")).build();
    let host = adw::EntryRow::builder().title(crate::i18n::t("IMAP 主机（可空）")).build();
    let insecure = adw::SwitchRow::builder()
        .title(crate::i18n::t("允许自签名证书"))
        .subtitle(crate::i18n::t("仅本地测试邮局 / TOFU"))
        .build();

    group.add(&email);
    group.add(&password);
    group.add(&host);
    group.add(&insecure);
    page.add(&group);

    let probe_group = adw::PreferencesGroup::new();
    probe_group.set_title(crate::i18n::t("连接测试"));
    let probe_label = gtk::Label::new(Some(crate::i18n::t("尚未测试")));
    probe_label.set_wrap(true);
    probe_label.set_xalign(0.0);
    probe_label.set_margin_start(12);
    probe_label.set_margin_end(12);
    probe_label.set_margin_top(8);
    probe_label.set_margin_bottom(8);
    probe_group.add(&probe_label);
    page.add(&probe_group);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(520);
    clamp.set_child(Some(&page));
    toolbar.set_content(Some(&clamp));

    let add = gtk::Button::with_label(crate::i18n::t("添加"));
    add.add_css_class("suggested-action");
    add.set_sensitive(false);
    header.pack_end(&add);

    let test = gtk::Button::with_label(crate::i18n::t("测试连接"));
    header.pack_start(&test);

    let add_for_enable = add.clone();
    email.connect_changed(move |row| {
        add_for_enable.set_sensitive(row.text().contains('@'));
    });

    let bridge_test = bridge.clone();
    let email_t = email.clone();
    let password_t = password.clone();
    let host_t = host.clone();
    let insecure_t = insecure.clone();
    let probe_label_t = probe_label.clone();
    test.connect_clicked(move |_| {
        let req = request_from_rows(&email_t, &password_t, &host_t, insecure_t.is_active());
        probe_label_t.set_text(crate::i18n::t("正在探测…"));
        let bridge = bridge_test.clone();
        let probe_label = probe_label_t.clone();
        crate::task::spawn_ui(
            move || match bridge.test_account(&req) {
                Ok(probe) => format_probe(&probe),
                Err(e) => crate::i18n::t(&e).to_owned(),
            },
            move |text| probe_label.set_text(&text),
        );
    });

    let bridge_add = bridge.clone();
    let email_a = email.clone();
    let password_a = password.clone();
    let host_a = host.clone();
    let insecure_a = insecure.clone();
    let dialog_a = dialog.clone();
    let on_added = std::rc::Rc::new(on_added);
    add.connect_clicked(move |_| {
        let on_added = on_added.clone();
        let req = request_from_rows(&email_a, &password_a, &host_a, insecure_a.is_active());
        let bridge = bridge_add.clone();
        let dialog = dialog_a.clone();
        crate::task::spawn_ui(
            move || bridge.add_account(req),
            move |result| match result {
                Ok(_) => {
                    dialog.close();
                    on_added();
                }
                Err(e) => {
                    dialog.set_title(&e);
                }
            },
        );
    });

    dialog.set_child(Some(&toolbar));
    dialog.present(Some(parent));
}

fn request_from_rows(
    email: &adw::EntryRow,
    password: &adw::PasswordEntryRow,
    host: &adw::EntryRow,
    insecure: bool,
) -> AddAccountRequest {
    let mut req = AddAccountRequest::new(email.text().to_string());
    let pw = password.text().to_string();
    if !pw.is_empty() {
        req.password = Some(pw);
    }
    let h = host.text().to_string();
    if !h.is_empty() {
        req.imap_host = Some(h);
    }
    req.accept_invalid_certs = insecure;
    req
}

fn format_probe(probe: &ConnectProbe) -> String {
    if probe.steps.is_empty() {
        return crate::i18n::t("无探测步骤").into();
    }
    probe
        .steps
        .iter()
        .map(|s| {
            let mark = match s.status {
                ProbeStatus::Ok => "✓",
                ProbeStatus::Failed => "✗",
                ProbeStatus::Skipped => "·",
            };
            format!("{mark} {:?}  {}", s.kind, s.detail)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
