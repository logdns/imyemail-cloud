use std::rc::Rc;
use std::sync::Arc;

use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
#[cfg(feature = "webkit")]
use webkit6::prelude::WebViewExt;

use chck_types::{Account, SendRequest};

use crate::engine_bridge::EngineBridge;
use crate::markdown::{self, Format};
use crate::util::Mailto;

pub fn open(
    app: &adw::Application,
    parent: &adw::ApplicationWindow,
    bridge: Arc<EngineBridge>,
    accounts: Vec<Account>,
    mailto: Option<Mailto>,
    on_sent: impl Fn(String) + 'static,
) {
    let win = adw::Window::new();
    win.set_application(Some(app));
    win.set_transient_for(Some(parent));
    win.set_title(Some(crate::i18n::t("写信 · Markdown")));
    win.set_default_width(760);
    win.set_default_height(640);

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let send_btn = gtk::Button::with_label(crate::i18n::t("发送"));
    send_btn.add_css_class("suggested-action");
    header.pack_end(&send_btn);
    toolbar.add_top_bar(&header);

    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::new();
    let from =
        gtk::DropDown::from_strings(&accounts.iter().map(|a| a.email.as_str()).collect::<Vec<_>>());
    let from_row = adw::ActionRow::builder().title(crate::i18n::t("发件人")).build();
    from_row.add_suffix(&from);
    group.add(&from_row);
    let to = adw::EntryRow::builder().title(crate::i18n::t("收件人")).build();
    let subject = adw::EntryRow::builder().title(crate::i18n::t("主题")).build();
    if let Some(m) = &mailto {
        to.set_text(&m.to);
        subject.set_text(&m.subject);
    }
    group.add(&to);
    group.add(&subject);
    page.add(&group);

    let edit = gtk::TextView::new();
    edit.set_wrap_mode(gtk::WrapMode::WordChar);
    edit.set_top_margin(16);
    edit.set_bottom_margin(16);
    edit.set_left_margin(16);
    edit.set_right_margin(16);
    edit.set_vexpand(true);
    edit.set_monospace(true);
    edit.add_css_class("markdown-editor");
    if let Some(m) = &mailto {
        edit.buffer().set_text(&m.body);
    }
    let edit_scroll = gtk::ScrolledWindow::new();
    edit_scroll.set_child(Some(&edit));
    edit_scroll.set_vexpand(true);

    let preview = gtk::Stack::new();
    preview.set_vexpand(true);
    #[cfg(feature = "webkit")]
    {
        let web = webkit6::WebView::new();
        if let Some(settings) = webkit6::prelude::WebViewExt::settings(&web) {
            settings.set_enable_javascript(false);
            settings.set_auto_load_images(false);
        }
        preview.add_named(&web, Some("preview"));
    }
    #[cfg(not(feature = "webkit"))]
    preview
        .add_named(&gtk::Label::new(Some(crate::i18n::t("预览需要 WebKitGTK"))), Some("preview"));
    preview.set_visible_child_name("preview");
    preview.set_visible(false);

    let stack = gtk::Stack::new();
    stack.set_vexpand(true);
    stack.add_named(&edit_scroll, Some("edit"));
    stack.add_named(&preview, Some("preview"));
    stack.set_visible_child_name("edit");

    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    controls.add_css_class("markdown-toolbar");
    controls.set_margin_start(12);
    controls.set_margin_end(12);
    controls.set_margin_top(8);
    controls.set_margin_bottom(8);
    for (label, format) in [
        ("H", Format::Heading),
        ("B", Format::Bold),
        ("I", Format::Italic),
        ("↗", Format::Link),
        ("•", Format::Bullet),
        ("1.", Format::Numbered),
        (">", Format::Quote),
        ("`", Format::Code),
    ] {
        let button = gtk::Button::with_label(label);
        button.set_tooltip_text(Some(match format {
            Format::Heading => "标题",
            Format::Bold => "粗体",
            Format::Italic => "斜体",
            Format::Link => "链接",
            Format::Bullet => "无序列表",
            Format::Numbered => "有序列表",
            Format::Quote => "引用",
            Format::Code => "代码",
        }));
        let edit = edit.clone();
        button.connect_clicked(move |_| apply_format(&edit, format));
        controls.append(&button);
    }
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    controls.append(&spacer);
    let import_btn = gtk::Button::with_label(crate::i18n::t("导入文档"));
    import_btn.set_tooltip_text(Some(crate::i18n::t(
        "导入 Markdown，或使用本机 Microsoft MarkItDown 转换文档",
    )));
    controls.append(&import_btn);
    let edit_btn = gtk::ToggleButton::with_label(crate::i18n::t("编辑"));
    edit_btn.set_active(true);
    let preview_btn = gtk::ToggleButton::with_label(crate::i18n::t("预览"));
    preview_btn.set_group(Some(&edit_btn));
    controls.append(&edit_btn);
    controls.append(&preview_btn);
    {
        let stack = stack.clone();
        let preview = preview.clone();
        let edit = edit.clone();
        preview_btn.connect_toggled(move |button| {
            let showing = button.is_active();
            stack.set_visible_child_name(if showing { "preview" } else { "edit" });
            preview.set_visible(showing);
            if showing {
                update_preview(&edit, &preview);
            }
        });
    }

    {
        let edit = edit.clone();
        let win = win.clone();
        import_btn.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::new();
            let edit = edit.clone();
            let owner = win.clone();
            let callback_win = win.clone();
            dialog.open(Some(&owner), None::<&gtk::gio::Cancellable>, move |result| {
                let Ok(file) = result else {
                    return;
                };
                let Some(path) = file.path() else {
                    callback_win.set_title(Some(crate::i18n::t("无法读取远程文件")));
                    return;
                };
                crate::task::spawn_ui(
                    move || import_file(&path),
                    move |result| match result {
                        Ok(markdown) => {
                            let buffer = edit.buffer();
                            let current =
                                buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                            let separator = if current.trim().is_empty() { "" } else { "\n\n" };
                            buffer.set_text(&format!("{current}{separator}{markdown}"));
                        }
                        Err(error) => callback_win.set_title(Some(crate::i18n::t(&error))),
                    },
                );
            });
        });
    }

    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
    box_.append(&page);
    box_.append(&controls);
    box_.append(&stack);
    toolbar.set_content(Some(&box_));
    win.set_content(Some(&toolbar));

    let on_sent = Rc::new(on_sent);
    let submit = Rc::new({
        let to = to.clone();
        let subject = subject.clone();
        let body = edit.clone();
        let from = from.clone();
        let accounts = accounts.clone();
        let bridge = bridge.clone();
        let win = win.clone();
        let on_sent = on_sent.clone();
        move || {
            let to_text = to.text().to_string();
            if to_text.trim().is_empty() {
                win.set_title(Some(crate::i18n::t("请填写收件人")));
                return;
            }
            let idx = from.selected() as usize;
            let Some(account) = accounts.get(idx) else {
                win.set_title(Some(crate::i18n::t("没有账号")));
                return;
            };
            let buf = body.buffer();
            let text = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
            let html = markdown::render(&text);
            let req = SendRequest {
                account_id: account.id.clone(),
                to: vec![to_text],
                subject: subject.text().to_string(),
                body_text: text,
                body_html: Some(html),
                undo_window_secs: 10,
                ..SendRequest::default()
            };
            let bridge = bridge.clone();
            let win = win.clone();
            let on_sent = on_sent.clone();
            crate::task::spawn_ui(
                move || bridge.send(req),
                move |result| match result {
                    Ok(id) => {
                        win.close();
                        on_sent(id);
                    }
                    Err(e) => win.set_title(Some(crate::i18n::t(&e))),
                },
            );
        }
    });
    let submit_btn = submit.clone();
    send_btn.connect_clicked(move |_| submit_btn());
    let submit_key = submit.clone();
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, modifier| {
        if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            && (key == gtk::gdk::Key::Return || key == gtk::gdk::Key::KP_Enter)
        {
            submit_key();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    win.add_controller(keys);
    win.present();
}

fn apply_format(view: &gtk::TextView, format: Format) {
    let buffer = view.buffer();
    let (start, end) = buffer.selection_bounds().unwrap_or_else(|| {
        let cursor = buffer.cursor_position();
        (buffer.iter_at_offset(cursor), buffer.iter_at_offset(cursor))
    });
    let source = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
    let (replacement, cursor, length) =
        markdown::apply_format(&source, start.offset() as usize, end.offset() as usize, format);
    buffer.set_text(&replacement);
    let cursor = buffer.iter_at_offset(cursor as i32);
    let end =
        buffer.iter_at_offset((cursor.offset() + length as i32).min(buffer.end_iter().offset()));
    buffer.select_range(&cursor, &end);
    view.grab_focus();
}

fn update_preview(edit: &gtk::TextView, preview: &gtk::Stack) {
    let buffer = edit.buffer();
    let source = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
    let html = crate::content_policy::document(&markdown::render(&source), false);
    #[cfg(feature = "webkit")]
    if let Some(web) = preview.first_child().and_downcast::<webkit6::WebView>() {
        web.load_html(&html, None);
    }
    #[cfg(not(feature = "webkit"))]
    let _ = (preview, html);
}

fn import_file(path: &std::path::Path) -> Result<String, String> {
    let extension =
        path.extension().and_then(|value| value.to_str()).unwrap_or_default().to_ascii_lowercase();
    if matches!(extension.as_str(), "md" | "markdown" | "txt") {
        let bytes = std::fs::read(path).map_err(|_| "无法读取文件".to_string())?;
        if bytes.len() > 25 * 1024 * 1024 {
            return Err("文件超过 25 MiB".into());
        }
        return String::from_utf8(bytes).map_err(|_| "文件不是有效的 UTF-8 文本".into());
    }
    // Keep the conversion process local and explicit.  Linux distributions may
    // provide the Microsoft MarkItDown package as `python3 -m markitdown`, or
    // an administrator may point IMYEMAIL_CLOUD_MARKITDOWN at a pinned wrapper binary.
    let mut command = if let Ok(binary) = std::env::var("IMYEMAIL_CLOUD_MARKITDOWN") {
        std::process::Command::new(binary)
    } else {
        let mut command = std::process::Command::new("python3");
        command.args(["-m", "markitdown"]);
        command
    };
    let output = command
        .arg(path)
        .output()
        .map_err(|_| "未找到 Microsoft MarkItDown（请安装后重试）".to_string())?;
    if !output.status.success() {
        return Err("Microsoft MarkItDown 转换失败".into());
    }
    let markdown =
        String::from_utf8(output.stdout).map_err(|_| "转换结果不是有效的 UTF-8".to_string())?;
    if markdown.len() > 25 * 1024 * 1024 {
        return Err("转换结果超过 25 MiB".into());
    }
    if markdown.trim().is_empty() {
        return Err("文档没有可提取文字".into());
    }
    Ok(markdown)
}
