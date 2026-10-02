use chck_types::Address;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedHeaders {
    pub subject: String,
    pub from: Vec<Address>,
    pub to: Vec<Address>,
    pub cc: Vec<Address>,
    pub date_unix: i64,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub has_attachments: bool,
}

#[must_use]
pub fn parse_headers(raw: &str) -> ParsedHeaders {
    let mut out = ParsedHeaders::default();
    let mut current: Option<(String, String)> = None;
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in raw.replace("\r\n", "\n").lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some((_, ref mut v)) = current {
                v.push(' ');
                v.push_str(line.trim());
            }
            continue;
        }
        if let Some(pair) = current.take() {
            fields.push(pair);
        }
        if let Some((name, value)) = line.split_once(':') {
            current = Some((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    if let Some(pair) = current {
        fields.push(pair);
    }
    for (name, value) in fields {
        match name.as_str() {
            "subject" => out.subject = decode_atom(&value),
            "from" => out.from = parse_address_list(&value),
            "to" => out.to = parse_address_list(&value),
            "cc" => out.cc = parse_address_list(&value),
            "date" => out.date_unix = parse_rfc2822_date(&value).unwrap_or(0),
            "message-id" => out.message_id = Some(value),
            "in-reply-to" => out.in_reply_to = Some(value),
            "content-type" if value.to_ascii_lowercase().contains("multipart") => {
                out.has_attachments = true;
            }
            "content-disposition" if value.to_ascii_lowercase().contains("attachment") => {
                out.has_attachments = true;
            }
            _ => {}
        }
    }
    out
}

#[must_use]
pub fn parse_address_list(raw: &str) -> Vec<Address> {
    let Ok(list) = mailparse::addrparse(raw) else { return Vec::new() };
    list.iter()
        .flat_map(|address| match address {
            mailparse::MailAddr::Single(a) => vec![a.clone()],
            mailparse::MailAddr::Group(g) => g.addrs.clone(),
        })
        .map(|a| Address { name: a.display_name.map(|name| decode_atom(&name)), email: a.addr })
        .collect()
}

fn decode_atom(s: &str) -> String {
    let raw = format!("Subject: {s}");
    mailparse::parse_header(raw.as_bytes())
        .map(|(h, _)| h.get_value())
        .unwrap_or_else(|_| s.to_string())
}

#[must_use]
pub fn parse_rfc2822_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let rest = s.split_once(", ").map_or(s, |(_, r)| r);
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    let day: u32 = parts[0].parse().ok()?;
    let month = month_num(parts[1])?;
    let year: i32 = parts[2].parse().ok()?;
    let (hour, min, sec) = {
        let t: Vec<u32> = parts[3].split(':').filter_map(|n| n.parse().ok()).collect();
        (
            t.first().copied().unwrap_or(0),
            t.get(1).copied().unwrap_or(0),
            t.get(2).copied().unwrap_or(0),
        )
    };
    let tz = parts.get(4).copied().unwrap_or("+0000");
    let tz_secs = parse_tz(tz).unwrap_or(0);
    let days = days_from_civil(year, month, day)?;
    let utc =
        i64::from(days) * 86400 + i64::from(hour) * 3600 + i64::from(min) * 60 + i64::from(sec)
            - i64::from(tz_secs);
    Some(utc)
}

fn parse_tz(tz: &str) -> Option<i32> {
    let sign = if tz.starts_with('-') { -1 } else { 1 };
    let digits = tz.trim_start_matches(['+', '-']);
    if digits.len() < 4 {
        return None;
    }
    let h: i32 = digits.get(..2)?.parse().ok()?;
    let m: i32 = digits.get(2..4)?.parse().ok()?;
    Some(sign * (h * 3600 + m * 60))
}

fn month_num(m: &str) -> Option<u32> {
    Some(match &m.get(..3)?.to_ascii_lowercase()[..] {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    })
}

fn days_from_civil(y: i32, m: u32, d: u32) -> Option<i32> {
    if !(1..=12).contains(&m) || d == 0 || d > 31 {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + i32::try_from(doy).ok()?;
    Some(era * 146097 + doe - 719468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_folded_reply_and_quoted_address_from_golden() {
        let h = parse_headers(include_str!("../tests/fixtures/encoded-reply.eml"));
        assert_eq!(h.subject, "Re: 测试回复");
        assert_eq!(h.from[0].name.as_deref(), Some("测试"));
        assert_eq!(h.to.len(), 1);
        assert_eq!(h.to[0].name.as_deref(), Some("Doe, Jane"));
        assert_eq!(h.to[0].email, "jane@imyemail.test");
    }

    #[test]
    fn parses_seed_headers() {
        let h = parse_headers(
            "From: Alice <alice@imyemail.test>\r\n\
             To: Dev <dev@imyemail.test>\r\n\
             Subject: Hello from testkit\r\n\
             Message-ID: <simple-001@imyemail.test>\r\n\
             Date: Tue, 15 Sep 2026 10:00:00 +0800\r\n",
        );
        assert_eq!(h.subject, "Hello from testkit");
        assert_eq!(h.from[0].email, "alice@imyemail.test");
        assert_eq!(h.date_unix, 1_789_437_600);
    }
}
