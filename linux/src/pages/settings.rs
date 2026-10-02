use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use crate::layout::{APP_ID, BRAND, SUPPORT_URL, TERMS_URL};
use crate::secrets;

pub fn present(parent: &impl IsA<gtk::Widget>, on_remote_images: impl Fn(bool) + 'static) {
    let dialog = adw::Dialog::new();
    dialog.set_title(crate::i18n::t("设置"));
    dialog.set_content_width(520);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    let page = adw::PreferencesPage::new();

    let appearance = adw::PreferencesGroup::new();
    appearance.set_title(crate::i18n::t("外观"));
    let language = adw::ComboRow::builder()
        .title(crate::i18n::t("语言"))
        .subtitle(crate::i18n::t("更改语言将在下次启动时生效。"))
        .build();
    language.set_model(Some(&gtk::StringList::new(&crate::i18n::NAMES)));
    language.set_selected(
        crate::i18n::CODES.iter().position(|v| *v == crate::i18n::load().language).unwrap_or(0)
            as u32,
    );
    language.connect_selected_notify(|row| {
        let mut prefs = crate::i18n::load();
        prefs.language = crate::i18n::CODES[row.selected() as usize].into();
        if let Err(error) = crate::i18n::save(&prefs) {
            row.set_subtitle(&error);
        }
    });
    appearance.add(&language);
    let scheme = adw::ComboRow::builder().title(crate::i18n::t("颜色")).build();
    scheme.set_model(Some(&gtk::StringList::new(&[
        crate::i18n::t("跟随系统"),
        crate::i18n::t("浅色"),
        crate::i18n::t("深色"),
    ])));
    scheme.set_selected(match crate::i18n::load().theme.as_str() {
        "light" => 1,
        "dark" => 2,
        _ => 0,
    });
    scheme.connect_selected_notify(move |row| {
        let manager = adw::StyleManager::default();
        manager.set_color_scheme(match row.selected() {
            1 => adw::ColorScheme::ForceLight,
            2 => adw::ColorScheme::ForceDark,
            _ => adw::ColorScheme::Default,
        });
        let mut prefs = crate::i18n::load();
        prefs.theme = match row.selected() {
            1 => "light",
            2 => "dark",
            _ => "system",
        }
        .into();
        if let Err(error) = crate::i18n::save(&prefs) {
            row.set_subtitle(&error);
        }
    });
    appearance.add(&scheme);
    page.add(&appearance);
    let privacy = adw::PreferencesGroup::new();
    privacy.set_title(crate::i18n::t("安全与隐私"));
    let remote = adw::SwitchRow::builder()
        .title(crate::i18n::t("远程图片"))
        .subtitle(crate::i18n::t("远程内容默认拦截，防止追踪像素。"))
        .active(crate::i18n::load().allow_remote_images)
        .build();
    remote.connect_active_notify(move |row| {
        let value = row.is_active();
        let mut prefs = crate::i18n::load();
        prefs.allow_remote_images = value;
        if let Err(error) = crate::i18n::save(&prefs) {
            row.set_subtitle(&error);
            return;
        }
        on_remote_images(value);
    });
    privacy.add(&remote);
    let secret_row = adw::ActionRow::builder()
        .title(crate::i18n::t("凭据存储"))
        .subtitle(if secrets::available() {
            "libsecret（Secret Service）"
        } else {
            "Local mail.db.secrets sidecar (0600)"
        })
        .build();
    privacy.add(&secret_row);
    page.add(&privacy);

    let about = adw::PreferencesGroup::new();
    about.set_title(crate::i18n::t("关于"));
    let id_row =
        adw::ActionRow::builder().title(crate::i18n::t("应用 ID")).subtitle(APP_ID).build();
    let brand_row = adw::ActionRow::builder().title(crate::i18n::t("产品")).subtitle(BRAND).build();
    about.add(&id_row);
    about.add(&brand_row);
    about.add(&gtk::LinkButton::with_label("https://imy.email", crate::i18n::t("官方网站")));
    page.add(&about);

    let clamp = adw::Clamp::new();
    clamp.set_child(Some(&page));
    toolbar.set_content(Some(&clamp));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(parent));
}

pub fn about(parent: &impl IsA<gtk::Widget>) {
    let dialog = adw::AboutDialog::builder()
        .application_name(BRAND)
        .application_icon(APP_ID)
        .developer_name("imyemail-cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .website("https://imy.email")
        .issue_url(SUPPORT_URL)
        .comments(crate::i18n::t("一次收件，处处原生。"))
        .build();
    dialog.add_link(crate::i18n::t("法律信息"), TERMS_URL);
    dialog.present(Some(parent));
}
