use chck_types::{Address, Folder, FolderRole, MessageSummary};
use std::path::PathBuf;

#[cfg_attr(not(feature = "gtk"), allow(dead_code))]
pub fn default_db() -> PathBuf {
    if let Ok(p) = std::env::var("IMYEMAIL_CLOUD_DB") {
        return PathBuf::from(p);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/imyemail-cloud/mail.db")
}

pub fn from_display(addrs: &[Address]) -> String {
    addrs
        .first()
        .map(|a| {
            a.name.as_deref().filter(|n| !n.is_empty()).unwrap_or(a.email.as_str()).to_string()
        })
        .unwrap_or_else(|| crate::i18n::t("无发件人").into())
}

#[cfg_attr(not(feature = "gtk"), allow(dead_code))]
pub fn from_email(addrs: &[Address]) -> String {
    addrs.first().map(|a| a.email.clone()).unwrap_or_default()
}

pub fn folder_label(folder: &Folder) -> String {
    match folder.role {
        FolderRole::Inbox => crate::i18n::t("收件箱").into(),
        FolderRole::Sent => crate::i18n::t("已发送").into(),
        FolderRole::Drafts => crate::i18n::t("草稿").into(),
        FolderRole::Trash => crate::i18n::t("废纸篓").into(),
        FolderRole::Junk => crate::i18n::t("垃圾").into(),
        FolderRole::Archive => crate::i18n::t("归档").into(),
        FolderRole::Flagged => crate::i18n::t("星标").into(),
        FolderRole::All => crate::i18n::t("全部").into(),
        FolderRole::Custom => {
            if folder.path.is_empty() {
                folder.remote_name.clone()
            } else {
                folder.path.clone()
            }
        }
    }
}

#[cfg_attr(not(feature = "gtk"), allow(dead_code))]
pub fn format_unix(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(ts);
    let delta = now.saturating_sub(ts);
    if delta < 60 {
        crate::i18n::t("刚刚").into()
    } else if delta < 3600 {
        crate::i18n::format("XPH0X 分钟前", &[(delta / 60).to_string()])
    } else if delta < 86_400 {
        crate::i18n::format("XPH0X 小时前", &[(delta / 3600).to_string()])
    } else if delta < 86_400 * 7 {
        crate::i18n::format("XPH0X 天前", &[(delta / 86400).to_string()])
    } else {
        let days = ts.div_euclid(86_400);
        let rem = ts.rem_euclid(86_400);
        let hour = rem / 3600;
        let min = (rem % 3600) / 60;
        let (y, m, d) = civil_from_days(days);
        format!("{y:04}-{m:02}-{d:02} {hour:02}:{min:02}")
    }
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + i64::from(m <= 2);
    (y as i32, m as u32, d as u32)
}

#[allow(dead_code)]
pub fn unread_dot(msg: &MessageSummary) -> &'static str {
    if msg.flags.seen { "" } else { "●" }
}

pub fn parse_mailto(uri: &str) -> Mailto {
    let raw = uri.strip_prefix("mailto:").unwrap_or(uri);
    let (addr, query) = raw.split_once('?').unwrap_or((raw, ""));
    let mut mail = Mailto { to: url_decode(addr), subject: String::new(), body: String::new() };
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k.to_ascii_lowercase().as_str() {
                "subject" => mail.subject = url_decode(v),
                "body" => mail.body = url_decode(v),
                _ => {}
            }
        }
    }
    mail
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mailto {
    pub to: String,
    pub subject: String,
    pub body: String,
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(c) = u8::from_str_radix(hex, 16) {
                    out.push(c as char);
                    i += 3;
                } else {
                    out.push('%');
                    i += 1;
                }
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mailto_parses_query() {
        let m = parse_mailto("mailto:dev@imy.email?subject=Hi%20there&body=Hello");
        assert_eq!(m.to, "dev@imy.email");
        assert_eq!(m.subject, "Hi there");
        assert_eq!(m.body, "Hello");
    }

    #[test]
    fn folder_inbox_zh() {
        let f = Folder {
            id: "a:INBOX".into(),
            account_id: "a".into(),
            remote_name: "INBOX".into(),
            path: "INBOX".into(),
            role: FolderRole::Inbox,
            unread_count: 0,
            total_count: 0,
        };
        assert_eq!(folder_label(&f), crate::i18n::t("收件箱"));
    }

    #[test]
    fn from_prefers_name() {
        let addr = Address { name: Some("Ada".into()), email: "ada@imy.email".into() };
        assert_eq!(from_display(&[addr]), "Ada");
    }
}
