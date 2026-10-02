use gtk::gio;
use gtk::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use crate::layout::{APP_ID, BRAND};

pub fn run() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|_| {
        gtk::Window::set_default_icon_name(crate::layout::APP_ID);
        let manager = adw::StyleManager::default();
        manager.set_color_scheme(match crate::i18n::load().theme.as_str() {
            "light" => adw::ColorScheme::ForceLight,
            "dark" => adw::ColorScheme::ForceDark,
            _ => adw::ColorScheme::Default,
        });
        let provider = gtk::CssProvider::new();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        load_css(&provider);
        manager.connect_dark_notify(move |_| load_css(&provider));
    });

    app.connect_activate(|app| {
        crate::window::present(app, None);
    });

    app.connect_open(|app, files, _hint| {
        let mailto = files.iter().find_map(|f| {
            let uri = f.uri();
            uri.starts_with("mailto:").then(|| crate::util::parse_mailto(&uri))
        });
        crate::window::present(app, mailto);
    });

    let argv: Vec<String> = std::env::args().filter(|a| a != "gui" && a != "--gui").collect();
    app.run_with_args(&argv);
}

fn load_css(provider: &gtk::CssProvider) {
    let dark = adw::StyleManager::default().is_dark();
    let colors = if dark {
        "@define-color accent_color #6B8AFF; @define-color accent_bg_color #6B8AFF; @define-color accent_fg_color #102A70;"
    } else {
        "@define-color accent_color #3D6BFE; @define-color accent_bg_color #3D6BFE; @define-color accent_fg_color white;"
    };
    provider.load_from_string(&format!("{colors}\n{}", include_str!("../data/style.css")));
}

pub fn register_actions(app: &adw::Application, window: &adw::ApplicationWindow) {
    let quit = gio::SimpleAction::new("quit", None);
    let app_quit = app.clone();
    quit.connect_activate(move |_, _| app_quit.quit());
    app.add_action(&quit);
    app.set_accels_for_action("app.quit", &["<Control>q"]);

    window.set_title(Some(BRAND));
}
