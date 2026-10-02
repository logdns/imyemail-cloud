use crate::ids::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub account_id: Option<AccountId>,
    pub email: String,
    pub name: Option<String>,
    #[serde(default)]
    pub vip: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarInvite {
    pub uid: String,
    pub summary: String,
    pub organizer: Option<String>,
    pub dtstart: Option<String>,
    pub dtend: Option<String>,
    pub method: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rsvp {
    Accept,
    Decline,
    Tentative,
}

impl Rsvp {
    #[must_use]
    pub fn as_ics(self) -> &'static str {
        match self {
            Self::Accept => "ACCEPTED",
            Self::Decline => "DECLINED",
            Self::Tentative => "TENTATIVE",
        }
    }
}

#[must_use]
pub fn parse_ics(raw: &str) -> CalendarInvite {
    let mut inv = CalendarInvite {
        uid: String::new(),
        summary: String::new(),
        organizer: None,
        dtstart: None,
        dtend: None,
        method: None,
    };
    for line in unfold_ics(raw) {
        let upper = line.to_ascii_uppercase();
        if let Some(v) = ics_value(&line, &upper, "UID:") {
            inv.uid = v;
        } else if let Some(v) = ics_value(&line, &upper, "SUMMARY") {
            inv.summary = v;
        } else if let Some(v) = ics_value(&line, &upper, "ORGANIZER") {
            inv.organizer = Some(strip_mailto(&v));
        } else if let Some(v) = ics_value(&line, &upper, "DTSTART") {
            inv.dtstart = Some(v);
        } else if let Some(v) = ics_value(&line, &upper, "DTEND") {
            inv.dtend = Some(v);
        } else if let Some(v) = ics_value(&line, &upper, "METHOD:") {
            inv.method = Some(v);
        }
    }
    inv
}

#[must_use]
pub fn rsvp_ics(invite: &CalendarInvite, rsvp: Rsvp, attendee: &str) -> String {
    format!(
        "BEGIN:VCALENDAR\r\nMETHOD:REPLY\r\nBEGIN:VEVENT\r\nUID:{}\r\nSUMMARY:{}\r\nATTENDEE;PARTSTAT={}:MAILTO:{}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        invite.uid,
        invite.summary,
        rsvp.as_ics(),
        attendee
    )
}

pub fn parse_vcard(raw: &str) -> Vec<Contact> {
    let mut out = Vec::new();
    let mut email = String::new();
    let mut name: Option<String> = None;
    let mut in_card = false;
    for raw_line in raw.lines() {
        let line = raw_line.trim();
        let u = line.to_ascii_uppercase();
        if u.starts_with("BEGIN:VCARD") {
            in_card = true;
            email.clear();
            name = None;
            continue;
        }
        if !in_card {
            continue;
        }
        if u.starts_with("END:VCARD") {
            if !email.is_empty() {
                out.push(Contact {
                    id: String::new(),
                    account_id: None,
                    email: email.clone(),
                    name: name.take(),
                    vip: false,
                });
            }
            in_card = false;
            continue;
        }
        if u.starts_with("EMAIL") {
            if let Some(v) = line.split_once(':').map(|(_, v)| v.trim().to_string())
                && v.contains('@')
            {
                email = v;
            }
        } else if u.starts_with("FN:") {
            name = Some(line[3..].trim().to_string());
        } else if u.starts_with("N:") && name.is_none() {
            let n = line[2..].trim();
            let parts: Vec<&str> = n.split(';').collect();
            let formatted = match (parts.first(), parts.get(1)) {
                (Some(last), Some(first)) if !first.is_empty() => {
                    format!("{} {}", first, last).trim().to_string()
                }
                (Some(last), _) => (*last).to_string(),
                _ => n.to_string(),
            };
            if !formatted.is_empty() {
                name = Some(formatted);
            }
        }
    }
    out
}

pub fn parse_csv(raw: &str) -> Vec<Contact> {
    let mut out = Vec::new();
    let mut lines = raw.lines();
    let header = lines.next().unwrap_or_default().to_ascii_lowercase();
    let cols: Vec<&str> = header.split(',').map(str::trim).collect();
    let email_i = cols.iter().position(|c| *c == "email" || *c == "e-mail").unwrap_or(0);
    let name_i = cols.iter().position(|c| *c == "name" || *c == "fn");
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        let email = fields.get(email_i).copied().unwrap_or("").to_string();
        if !email.contains('@') {
            continue;
        }
        let name =
            name_i.and_then(|i| fields.get(i).map(|s| (*s).to_string())).filter(|s| !s.is_empty());
        out.push(Contact { id: String::new(), account_id: None, email, name, vip: false });
    }
    out
}

fn unfold_ics(raw: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for line in raw.replace("\r\n", "\n").lines() {
        if (line.starts_with(' ') || line.starts_with('\t'))
            && let Some(last) = lines.last_mut()
        {
            last.push_str(line.trim_start());
        } else {
            lines.push(line.to_string());
        }
    }
    lines
}

fn ics_value(line: &str, upper: &str, key: &str) -> Option<String> {
    let k = key.to_ascii_uppercase();
    if !upper.starts_with(k.trim_end_matches(':')) {
        return None;
    }
    let v = line.split_once(':')?.1.trim();
    if v.is_empty() {
        return None;
    }
    Some(v.to_string())
}

fn strip_mailto(v: &str) -> String {
    v.split("MAILTO:").last().unwrap_or(v).split("mailto:").last().unwrap_or(v).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_invite_and_rsvp() {
        let ics = "BEGIN:VCALENDAR\r\nMETHOD:REQUEST\r\nBEGIN:VEVENT\r\nUID:abc\r\nSUMMARY:Standup\r\nORGANIZER:MAILTO:a@ex.com\r\nDTSTART:20260916T090000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let inv = parse_ics(ics);
        assert_eq!(inv.uid, "abc");
        assert_eq!(inv.summary, "Standup");
        assert_eq!(inv.organizer.as_deref(), Some("a@ex.com"));
        let reply = rsvp_ics(&inv, Rsvp::Accept, "me@ex.com");
        assert!(reply.contains("PARTSTAT=ACCEPTED"));
        assert!(reply.contains("UID:abc"));
    }

    #[test]
    fn parses_vcard_and_csv() {
        let cards = parse_vcard("BEGIN:VCARD\nFN:Alice\nEMAIL:alice@ex.com\nEND:VCARD\n");
        assert_eq!(cards[0].email, "alice@ex.com");
        assert_eq!(cards[0].name.as_deref(), Some("Alice"));
        let csv = parse_csv("email,name\nbob@ex.com,Bob\n");
        assert_eq!(csv[0].email, "bob@ex.com");
        assert_eq!(csv[0].name.as_deref(), Some("Bob"));
    }
}
