use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use adw::prelude::*;
use gtk::gio;
use gtk4 as gtk;
use libadwaita as adw;

use chck_types::{
    Account, AttachmentHandle, Folder, FolderId, FolderRole, MessageBody, MessageSummary,
};
#[cfg(feature = "webkit")]
use webkit6::prelude::*;

use crate::engine_bridge::EngineBridge;
use crate::layout::{APP_ID, BRAND, LIST_MAX, LIST_MIN, LIST_WIDTH, NAV_MAX, NAV_MIN, NAV_WIDTH};
use crate::task::spawn_ui;
use crate::util::{Mailto, format_unix, from_display};
use crate::widgets::{folder_row, mail_row, nav_row};

struct Mailbox {
    bridge: Arc<EngineBridge>,
    accounts: Vec<Account>,
    folders: Vec<Folder>,
    messages: Vec<MessageSummary>,
    selected_folder: Option<FolderId>,
    unified: bool,
    allow_remote_images: bool,
    current_body: Option<MessageBody>,
}

struct Widgets {
    window: adw::ApplicationWindow,
    toast: adw::ToastOverlay,
    nav: gtk::ListBox,
    list: gtk::ListBox,
    subject: gtk::Label,
    meta: gtk::Label,
    banner: gtk::Label,
    #[cfg(not(feature = "webkit"))]
    body: gtk::TextView,
    #[cfg(feature = "webkit")]
    body_web: webkit6::WebView,
    attachments: gtk::Box,
    status: gtk::Label,
    search: gtk::SearchBar,
    search_entry: gtk::SearchEntry,
}

pub fn present(app: &adw::Application, mailto: Option<Mailto>) {
    let preferences = crate::i18n::load();
    let db = crate::util::default_db();
    if let Some(parent) = db.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let bridge = match EngineBridge::open(&db) {
        Ok(b) => Arc::new(b),
        Err(e) => {
            let win = adw::ApplicationWindow::builder()
                .application(app)
                .title(BRAND)
                .content(&gtk::Label::new(Some(&e)))
                .build();
            win.present();
            return;
        }
    };

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(BRAND)
        .default_width(1280)
        .default_height(820)
        .icon_name(APP_ID)
        .build();

    crate::app::register_actions(app, &window);

    let toast = adw::ToastOverlay::new();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let search_entry =
        gtk::SearchEntry::builder().placeholder_text(crate::i18n::t("搜索邮件")).build();
    let search = gtk::SearchBar::new();
    search.set_child(Some(&search_entry));
    search.set_key_capture_widget(Some(&window));

    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new(BRAND, crate::i18n::t("统一收件箱"));
    header.set_title_widget(Some(&title));

    let compose_btn = gtk::Button::from_icon_name("mail-message-new-symbolic");
    compose_btn.set_tooltip_text(Some(crate::i18n::t("写信 (Ctrl+N)")));
    header.pack_start(&compose_btn);

    let search_btn = gtk::ToggleButton::builder().icon_name("system-search-symbolic").build();
    search_btn.set_tooltip_text(Some(crate::i18n::t("搜索 (Ctrl+F)")));
    header.pack_end(&search_btn);

    let menu_btn =
        gtk::MenuButton::builder().icon_name("open-menu-symbolic").menu_model(&app_menu()).build();
    header.pack_end(&menu_btn);

    let add_btn = gtk::Button::from_icon_name("list-add-symbolic");
    add_btn.set_tooltip_text(Some(crate::i18n::t("添加账号")));
    header.pack_end(&add_btn);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.add_top_bar(&search);

    let nav = gtk::ListBox::new();
    nav.set_selection_mode(gtk::SelectionMode::Single);
    nav.add_css_class("navigation-sidebar");
    let nav_scroll = gtk::ScrolledWindow::new();
    nav_scroll.set_child(Some(&nav));
    nav_scroll.set_min_content_width(NAV_MIN);
    nav_scroll.set_max_content_width(NAV_MAX);
    nav_scroll.set_width_request(NAV_WIDTH);
    nav_scroll.set_vexpand(true);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("boxed-list");
    let list_scroll = gtk::ScrolledWindow::new();
    list_scroll.add_css_class("mail-list-scroll");
    list_scroll.set_child(Some(&list));
    list_scroll.set_min_content_width(LIST_MIN);
    list_scroll.set_max_content_width(LIST_MAX);
    list_scroll.set_width_request(LIST_WIDTH);
    list_scroll.set_vexpand(true);
    list_scroll.set_hexpand(true);

    let subject = gtk::Label::new(None);
    subject.set_xalign(0.0);
    subject.set_wrap(true);
    subject.add_css_class("title-2");
    subject.set_selectable(true);

    let meta = gtk::Label::new(None);
    meta.set_xalign(0.0);
    meta.add_css_class("dim-label");
    meta.set_wrap(true);
    meta.set_selectable(true);

    let banner = gtk::Label::new(None);
    banner.add_css_class("remote-banner");
    banner.set_xalign(0.0);
    banner.set_wrap(true);
    banner.set_visible(false);

    let body_scroll = gtk::ScrolledWindow::new();
    body_scroll.set_vexpand(true);
    body_scroll.set_hexpand(true);
    #[cfg(not(feature = "webkit"))]
    let body = {
        let body = gtk::TextView::new();
        body.set_editable(false);
        body.set_wrap_mode(gtk::WrapMode::WordChar);
        body.set_left_margin(8);
        body.set_right_margin(8);
        body.set_top_margin(8);
        body.set_bottom_margin(8);
        body.add_css_class("mail-body");
        body_scroll.set_child(Some(&body));
        body
    };
    #[cfg(feature = "webkit")]
    let body_web = {
        let web = webkit6::WebView::new();
        if let Some(settings) = webkit6::prelude::WebViewExt::settings(&web) {
            settings.set_auto_load_images(preferences.allow_remote_images);
            settings.set_enable_javascript(false);
        }
        body_scroll.set_child(Some(&web));
        web
    };
    let attachments = gtk::Box::new(gtk::Orientation::Vertical, 4);
    attachments.set_visible(false);

    let read_actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let reply_btn = gtk::Button::with_label(crate::i18n::t("回复"));
    let flag_btn = gtk::Button::with_label(crate::i18n::t("星标"));
    let delete_btn = gtk::Button::with_label(crate::i18n::t("删除"));
    delete_btn.add_css_class("destructive-action");
    read_actions.append(&reply_btn);
    read_actions.append(&flag_btn);
    read_actions.append(&delete_btn);

    let read_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    read_box.set_margin_start(16);
    read_box.set_margin_end(16);
    read_box.set_margin_top(12);
    read_box.set_margin_bottom(12);
    read_box.set_hexpand(true);
    read_box.append(&subject);
    read_box.append(&meta);
    read_box.append(&read_actions);
    read_box.append(&banner);
    read_box.append(&attachments);
    read_box.append(&body_scroll);

    let list_read = adw::NavigationSplitView::new();
    list_read.set_sidebar(Some(&adw::NavigationPage::new(&list_scroll, crate::i18n::t("邮件"))));
    list_read.set_content(Some(&adw::NavigationPage::new(&read_box, crate::i18n::t("阅读"))));
    list_read.set_min_sidebar_width(LIST_MIN as f64);
    list_read.set_max_sidebar_width(LIST_MAX as f64);
    list_read.set_sidebar_width_fraction(0.38);

    let split = adw::OverlaySplitView::new();
    split.set_sidebar(Some(&nav_scroll));
    split.set_content(Some(&list_read));
    split.set_min_sidebar_width(NAV_MIN as f64);
    split.set_max_sidebar_width(NAV_MAX as f64);
    split.set_show_sidebar(true);

    let status = gtk::Label::new(Some(crate::i18n::t("就绪")));
    status.add_css_class("dim-label");
    status.set_xalign(0.0);
    status.set_margin_start(12);
    status.set_margin_end(12);
    status.set_margin_top(4);
    status.set_margin_bottom(4);

    outer.append(&split);
    outer.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    outer.append(&status);
    toolbar.set_content(Some(&outer));
    toast.set_child(Some(&toolbar));
    window.set_content(Some(&toast));

    let widgets = Rc::new(Widgets {
        window: window.clone(),
        toast: toast.clone(),
        nav: nav.clone(),
        list: list.clone(),
        subject: subject.clone(),
        meta: meta.clone(),
        banner: banner.clone(),
        #[cfg(not(feature = "webkit"))]
        body: body.clone(),
        #[cfg(feature = "webkit")]
        body_web: body_web.clone(),
        attachments: attachments.clone(),
        status: status.clone(),
        search: search.clone(),
        search_entry: search_entry.clone(),
    });
    let state = Rc::new(RefCell::new(Mailbox {
        bridge: bridge.clone(),
        accounts: Vec::new(),
        folders: Vec::new(),
        messages: Vec::new(),
        selected_folder: None,
        unified: true,
        allow_remote_images: preferences.allow_remote_images,
        current_body: None,
    }));

    bind_shortcuts(&window, app);

    {
        let state = state.clone();
        let widgets = widgets.clone();
        let app = app.clone();
        compose_btn.connect_clicked(move |_| open_compose(&app, &state, &widgets, None));
    }
    {
        let widgets = widgets.clone();
        search_btn.connect_toggled(move |btn| {
            widgets.search.set_search_mode(btn.is_active());
        });
    }
    {
        let state = state.clone();
        let widgets = widgets.clone();
        add_btn.connect_clicked(move |_| open_add(&state, &widgets));
    }
    {
        let state = state.clone();
        let widgets = widgets.clone();
        nav.connect_row_activated(move |_, row| on_nav(&state, &widgets, row.index()));
    }
    {
        let state = state.clone();
        let widgets = widgets.clone();
        list.connect_row_activated(move |_, row| on_message(&state, &widgets, row.index()));
    }
    {
        let state = state.clone();
        let widgets = widgets.clone();
        search_entry.connect_activate(move |entry| {
            let q = entry.text().to_string();
            run_search(&state, &widgets, q);
        });
    }
    {
        let state = state.clone();
        let widgets = widgets.clone();
        let app = app.clone();
        reply_btn.connect_clicked(move |_| reply(&app, &state, &widgets));
    }
    {
        let state = state.clone();
        let widgets = widgets.clone();
        flag_btn.connect_clicked(move |_| toggle_flag(&state, &widgets));
    }
    {
        let state = state.clone();
        let widgets = widgets.clone();
        delete_btn.connect_clicked(move |_| delete_selected(&state, &widgets));
    }

    let new_account = gio::SimpleAction::new("new-account", None);
    {
        let state = state.clone();
        let widgets = widgets.clone();
        new_account.connect_activate(move |_, _| open_add(&state, &widgets));
    }
    window.add_action(&new_account);

    let compose = gio::SimpleAction::new("compose", None);
    {
        let state = state.clone();
        let widgets = widgets.clone();
        let app = app.clone();
        compose.connect_activate(move |_, _| open_compose(&app, &state, &widgets, None));
    }
    window.add_action(&compose);

    let find = gio::SimpleAction::new("find", None);
    {
        let widgets = widgets.clone();
        find.connect_activate(move |_, _| {
            widgets.search.set_search_mode(true);
            widgets.search_entry.grab_focus();
        });
    }
    window.add_action(&find);

    let settings = gio::SimpleAction::new("settings", None);
    {
        let window = window.clone();
        let bridge = Arc::clone(&state.borrow().bridge);
        let state = state.clone();
        let widgets = widgets.clone();
        settings.connect_activate(move |_, _| {
            let state = state.clone();
            let widgets = widgets.clone();
            crate::pages::settings::present(&window, move |value| {
                let body = {
                    let mut state = state.borrow_mut();
                    state.allow_remote_images = value;
                    state.current_body.clone()
                };
                if let Some(body) = body {
                    show_body(&widgets, &body, value);
                }
            })
        });
    }
    window.add_action(&settings);

    let about = gio::SimpleAction::new("about", None);
    {
        let window = window.clone();
        about.connect_activate(move |_, _| crate::pages::settings::about(&window));
    }
    window.add_action(&about);

    let sync = gio::SimpleAction::new("sync", None);
    {
        let state = state.clone();
        let widgets = widgets.clone();
        sync.connect_activate(move |_, _| reload(&state, &widgets, true));
    }
    window.add_action(&sync);

    bootstrap(state.clone(), widgets.clone());

    if let Some(m) = mailto {
        open_compose(app, &state, &widgets, Some(m));
    }

    window.present();
}

fn app_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    menu.append(Some(crate::i18n::t("添加账号")), Some("win.new-account"));
    menu.append(Some(crate::i18n::t("立即同步")), Some("win.sync"));
    menu.append(Some(crate::i18n::t("设置")), Some("win.settings"));
    menu.append(Some(crate::i18n::t("关于 imyemail-cloud")), Some("win.about"));
    menu.append(Some(crate::i18n::t("退出")), Some("app.quit"));
    menu
}

fn bind_shortcuts(window: &adw::ApplicationWindow, app: &adw::Application) {
    app.set_accels_for_action("win.compose", &["<Control>n"]);
    app.set_accels_for_action("win.find", &["<Control>f", "slash"]);
    app.set_accels_for_action("win.sync", &["<Control>r"]);
    app.set_accels_for_action("win.settings", &["<Control>comma"]);
    let controller = gtk::ShortcutController::new();
    controller.set_scope(gtk::ShortcutScope::Global);
    window.add_controller(controller);
}

fn bootstrap(state: Rc<RefCell<Mailbox>>, widgets: Rc<Widgets>) {
    let bridge = state.borrow().bridge.clone();
    widgets.status.set_text(crate::i18n::t("加载中…"));
    spawn_ui(
        move || bridge.accounts(),
        move |accounts| match accounts {
            Ok(list) => {
                state.borrow_mut().accounts = list;
                if state.borrow().accounts.is_empty() {
                    open_add(&state, &widgets);
                    render_nav(&state, &widgets);
                    widgets.status.set_text(crate::i18n::t("添加一个账号开始使用"));
                } else {
                    reload(&state, &widgets, true);
                }
            }
            Err(e) => widgets.status.set_text(crate::i18n::t(&e)),
        },
    );
}

fn reload(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>, sync: bool) {
    load_cached(state, widgets);
    if sync {
        sync_in_background(state, widgets);
    }
}

/// Render the local store first. Network synchronization is deliberately
/// detached from this path so opening the app never looks stuck on “syncing”.
fn load_cached(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>) {
    let (bridge, accounts, selected, unified) = {
        let s = state.borrow();
        (s.bridge.clone(), s.accounts.clone(), s.selected_folder.clone(), s.unified)
    };
    if accounts.is_empty() {
        render_nav(state, widgets);
        return;
    }
    let state = state.clone();
    let widgets = widgets.clone();
    spawn_ui(
        move || {
            let mut folders = Vec::new();
            for acc in &accounts {
                if let Ok(mut fs) = bridge.folders(&acc.id) {
                    folders.append(&mut fs);
                }
            }
            let folder_id = selected.or_else(|| {
                folders
                    .iter()
                    .find(|f| f.role == FolderRole::Inbox)
                    .or_else(|| folders.first())
                    .map(|f| f.id.clone())
            });
            let messages = if unified {
                bridge.unified_inbox()
            } else if let Some(id) = &folder_id {
                bridge.messages(id)
            } else {
                Ok(Vec::new())
            };
            let _ = bridge.flush_due_sends();
            (folders, folder_id, messages)
        },
        move |(folders, folder_id, messages)| {
            let mut s = state.borrow_mut();
            s.folders = folders;
            s.selected_folder = folder_id;
            s.messages = messages.unwrap_or_default();
            drop(s);
            render_nav(&state, &widgets);
            render_list(&state, &widgets);
            widgets.status.set_text(crate::i18n::t("就绪"));
        },
    );
}

fn sync_in_background(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>) {
    let bridge = state.borrow().bridge.clone();
    let state = state.clone();
    let widgets = widgets.clone();
    // Keep the cached mailbox usable while the refresh runs; a network round
    // trip must not replace the ready state with a permanent-looking spinner.
    widgets.status.set_text(crate::i18n::t("就绪"));
    spawn_ui(
        move || {
            let _ = bridge.tick(None, false);
            bridge.poll_events()
        },
        move |events| {
            notify_new(&widgets, &events);
            widgets.status.set_text(crate::i18n::t("就绪"));
            load_cached(&state, &widgets);
        },
    );
}

fn render_nav(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>) {
    while let Some(child) = widgets.nav.first_child() {
        widgets.nav.remove(&child);
    }
    let s = state.borrow();
    widgets.nav.append(&nav_row(crate::i18n::t("统一收件箱"), None, Some("mail-inbox-symbolic")));
    widgets.nav.append(&nav_row(crate::i18n::t("星标"), None, Some("starred-symbolic")));
    let sep = nav_row("", None, None);
    sep.set_sensitive(false);
    sep.set_activatable(false);
    widgets.nav.append(&sep);
    for acc in &s.accounts {
        widgets.nav.append(&nav_row(&acc.email, None, Some("avatar-default-symbolic")));
        for folder in s.folders.iter().filter(|f| f.account_id == acc.id) {
            widgets.nav.append(&folder_row(folder));
        }
    }
    if s.accounts.is_empty() {
        widgets.nav.append(&nav_row(crate::i18n::t("添加账号…"), None, Some("list-add-symbolic")));
    }
}

fn render_list(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>) {
    while let Some(child) = widgets.list.first_child() {
        widgets.list.remove(&child);
    }
    let s = state.borrow();
    if s.messages.is_empty() {
        let empty = gtk::Label::new(Some(crate::i18n::t("全部处理完了")));
        empty.add_css_class("dim-label");
        empty.set_margin_top(24);
        widgets.list.append(&empty);
        return;
    }
    for msg in &s.messages {
        widgets.list.append(&mail_row(msg));
    }
}

fn on_nav(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>, index: i32) {
    if index <= 0 {
        state.borrow_mut().unified = true;
        state.borrow_mut().selected_folder = None;
        reload(state, widgets, true);
        return;
    }
    if index == 1 {
        let bridge = state.borrow().bridge.clone();
        state.borrow_mut().unified = false;
        let state = state.clone();
        let widgets = widgets.clone();
        spawn_ui(
            move || {
                bridge
                    .unified_inbox()
                    .map(|rows| rows.into_iter().filter(|m| m.flags.flagged).collect())
            },
            move |rows| {
                state.borrow_mut().messages = rows.unwrap_or_default();
                render_list(&state, &widgets);
            },
        );
        return;
    }
    let s = state.borrow();
    let mut cursor = 3i32;
    for acc in &s.accounts {
        cursor += 1;
        for folder in s.folders.iter().filter(|f| f.account_id == acc.id) {
            if cursor == index {
                let id = folder.id.clone();
                drop(s);
                state.borrow_mut().unified = false;
                state.borrow_mut().selected_folder = Some(id);
                reload(state, widgets, true);
                return;
            }
            cursor += 1;
        }
    }
}

fn on_message(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>, index: i32) {
    let (bridge, msg) = {
        let s = state.borrow();
        let Some(msg) = s.messages.get(index as usize).cloned() else {
            return;
        };
        (s.bridge.clone(), msg)
    };
    widgets.subject.set_text(&msg.subject);
    widgets.meta.set_text(&format!(
        "{}  ·  {}",
        from_display(&msg.from),
        format_unix(msg.date_unix)
    ));
    set_body_text(widgets, crate::i18n::t("加载正文…"));
    state.borrow_mut().current_body = None;
    clear_attachments(widgets);
    let id = msg.id.clone();
    let widgets = widgets.clone();
    let state = state.clone();
    let mut flags = msg.flags;
    flags.seen = true;
    spawn_ui(
        move || {
            let body = bridge.body(&id);
            let atts = bridge.attachments(&id).unwrap_or_default();
            let _ = bridge.set_flags(&id, flags);
            (body, atts)
        },
        move |(body, atts)| match body {
            Ok(body) => {
                let allow_remote_images = {
                    let mut state = state.borrow_mut();
                    state.current_body = Some(body.clone());
                    state.allow_remote_images
                };
                show_body(&widgets, &body, allow_remote_images);
                show_attachments(&widgets, &atts);
            }
            Err(e) => set_body_text(&widgets, crate::i18n::t(&e)),
        },
    );
}

fn show_body(widgets: &Widgets, body: &MessageBody, allow_remote_images: bool) {
    if body.remote_blocked > 0 && !allow_remote_images {
        widgets.banner.set_visible(true);
        widgets.banner.set_text(&crate::i18n::format(
            "已拦截 XPH0X 个远程内容 · 默认不加载远程图片",
            &[body.remote_blocked.to_string()],
        ));
    } else {
        widgets.banner.set_visible(false);
    }
    let text = if body.text.trim().is_empty() {
        strip_tags(&body.html_sanitized)
    } else {
        body.text.clone()
    };
    #[cfg(feature = "webkit")]
    {
        let html = if body.html_sanitized.trim().is_empty() {
            format!("<pre>{}</pre>", html_escape(&text))
        } else {
            body.html_sanitized.clone()
        };
        if let Some(settings) = webkit6::prelude::WebViewExt::settings(&widgets.body_web) {
            settings.set_auto_load_images(allow_remote_images);
        }
        widgets
            .body_web
            .load_html(&crate::content_policy::document(&html, allow_remote_images), None);
    }
    #[cfg(not(feature = "webkit"))]
    {
        widgets.body.buffer().set_text(&text);
    }
}

fn set_body_text(widgets: &Widgets, text: &str) {
    #[cfg(feature = "webkit")]
    widgets.body_web.load_html(&format!("<pre>{}</pre>", html_escape(text)), None);
    #[cfg(not(feature = "webkit"))]
    widgets.body.buffer().set_text(text);
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn clear_attachments(widgets: &Widgets) {
    while let Some(child) = widgets.attachments.first_child() {
        widgets.attachments.remove(&child);
    }
    widgets.attachments.set_visible(false);
}

fn show_attachments(widgets: &Widgets, atts: &[AttachmentHandle]) {
    clear_attachments(widgets);
    if atts.is_empty() {
        return;
    }
    widgets.attachments.set_visible(true);
    for att in atts {
        let label = gtk::Label::new(Some(&format!("{} ({})", att.name, att.size)));
        label.set_xalign(0.0);
        widgets.attachments.append(&label);
    }
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    html_unescape(&out)
}

fn html_unescape(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn open_add(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>) {
    let bridge = state.borrow().bridge.clone();
    let state = state.clone();
    let widgets = widgets.clone();
    let parent = widgets.window.clone();
    crate::pages::account::present(&parent, bridge, move || {
        let bridge = state.borrow().bridge.clone();
        if let Ok(accounts) = bridge.accounts() {
            state.borrow_mut().accounts = accounts;
        }
        reload(&state, &widgets, true);
    });
}

fn open_compose(
    app: &adw::Application,
    state: &Rc<RefCell<Mailbox>>,
    widgets: &Rc<Widgets>,
    mailto: Option<Mailto>,
) {
    let accounts = state.borrow().accounts.clone();
    if accounts.is_empty() {
        open_add(state, widgets);
        return;
    }
    let bridge = state.borrow().bridge.clone();
    let widgets = widgets.clone();
    let parent = widgets.window.clone();
    crate::pages::compose::open(app, &parent, bridge.clone(), accounts, mailto, move |draft_id| {
        let toast = adw::Toast::new(crate::i18n::t("已加入发件队列"));
        toast.set_button_label(Some(crate::i18n::t("撤销")));
        toast.set_timeout(10);
        let draft = draft_id.clone();
        let bridge = bridge.clone();
        let widgets_u = widgets.clone();
        toast.connect_button_clicked(move |_| {
            let draft = draft.clone();
            let bridge = bridge.clone();
            let widgets_u = widgets_u.clone();
            spawn_ui(
                move || bridge.undo_send(&draft).is_ok(),
                move |ok| {
                    widgets_u.status.set_text(if ok {
                        crate::i18n::t("已撤销发送")
                    } else {
                        crate::i18n::t("撤销失败")
                    });
                },
            );
        });
        widgets.toast.add_toast(toast);
    });
}

fn reply(app: &adw::Application, state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>) {
    let s = state.borrow();
    let Some(row) = widgets.list.selected_row() else {
        return;
    };
    let Some(msg) = s.messages.get(row.index() as usize) else {
        return;
    };
    let mailto = Mailto {
        to: crate::util::from_email(&msg.from),
        subject: if msg.subject.to_ascii_lowercase().starts_with("re:") {
            msg.subject.clone()
        } else {
            format!("Re: {}", msg.subject)
        },
        body: String::new(),
    };
    drop(s);
    open_compose(app, state, widgets, Some(mailto));
}

fn toggle_flag(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>) {
    let s = state.borrow();
    let Some(row) = widgets.list.selected_row() else {
        return;
    };
    let Some(msg) = s.messages.get(row.index() as usize).cloned() else {
        return;
    };
    let bridge = s.bridge.clone();
    drop(s);
    let widgets = widgets.clone();
    let mut flags = msg.flags;
    flags.flagged = !msg.flags.flagged;
    spawn_ui(
        move || bridge.set_flags(&msg.id, flags),
        move |_| widgets.status.set_text(crate::i18n::t("已更新星标")),
    );
}

fn delete_selected(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>) {
    let s = state.borrow();
    let Some(row) = widgets.list.selected_row() else {
        return;
    };
    let Some(msg) = s.messages.get(row.index() as usize).cloned() else {
        return;
    };
    let bridge = s.bridge.clone();
    drop(s);
    let state = state.clone();
    let widgets = widgets.clone();
    spawn_ui(move || bridge.delete_message(&msg.id), move |_| reload(&state, &widgets, false));
}

fn run_search(state: &Rc<RefCell<Mailbox>>, widgets: &Rc<Widgets>, q: String) {
    if q.trim().is_empty() {
        reload(state, widgets, false);
        return;
    }
    let bridge = state.borrow().bridge.clone();
    let state = state.clone();
    let widgets = widgets.clone();
    widgets.status.set_text(crate::i18n::t("搜索中…"));
    spawn_ui(
        move || bridge.search(&q),
        move |rows| match rows {
            Ok(msgs) => {
                state.borrow_mut().messages = msgs;
                render_list(&state, &widgets);
                widgets.status.set_text(crate::i18n::t("搜索完成"));
            }
            Err(e) => widgets.status.set_text(crate::i18n::t(&e)),
        },
    );
}

fn notify_new(widgets: &Widgets, events: &[chck_types::EngineEvent]) {
    use chck_types::EngineEvent;
    for ev in events {
        if let EngineEvent::NewMessages { count, top, .. } = ev {
            if *count == 0 {
                continue;
            }
            let title = if let Some(m) = top.first() {
                format!("{} · {}", from_display(&m.from), m.subject)
            } else {
                crate::i18n::format("XPH0X 封新邮件", &[count.to_string()])
            };
            let n = gio::Notification::new("imyemail-cloud");
            n.set_body(Some(&title));
            if let Some(app) = widgets.window.application() {
                app.send_notification(Some("new-mail"), &n);
            }
        }
    }
}
