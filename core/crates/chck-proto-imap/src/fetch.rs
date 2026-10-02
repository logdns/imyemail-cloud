use crate::headers::{ParsedHeaders, parse_headers};
use crate::session::CommandResult;
use chck_types::Flags;

#[derive(Debug, Clone)]
pub struct EnvelopeFetch {
    pub uid: u32,
    pub flags: Flags,
    pub size: u32,
    pub headers: ParsedHeaders,
}

#[must_use]
pub fn parse_fetch_untagged(result: &CommandResult) -> Vec<EnvelopeFetch> {
    let blob = result.untagged.join("\n");
    let mut out = Vec::new();
    let mut rest = blob.as_str();
    while let Some(idx) = rest.find("* ") {
        let slice = &rest[idx..];
        if let Some(item) = parse_one_fetch(slice) {
            out.push(item);
        }
        rest = &rest[idx + 2..];
    }
    out
}

fn parse_one_fetch(block: &str) -> Option<EnvelopeFetch> {
    let line = block.lines().next()?;
    if !line.to_ascii_uppercase().contains(" FETCH ") {
        return None;
    }
    let uid = extract_number(block, "UID ").or_else(|| extract_seq(line))?;
    let flags = Flags::from_imap(&extract_paren(block, "FLAGS ").unwrap_or_default());
    let size = extract_number(block, "RFC822.SIZE ").unwrap_or(0);
    let headers = extract_first_literal(block).map(|raw| parse_headers(&raw)).unwrap_or_default();
    Some(EnvelopeFetch { uid, flags, size, headers })
}

fn extract_seq(line: &str) -> Option<u32> {
    let rest = line.strip_prefix("* ")?;
    rest.split_whitespace().next()?.parse().ok()
}

fn extract_number(hay: &str, key: &str) -> Option<u32> {
    let upper = hay.to_ascii_uppercase();
    let key_u = key.to_ascii_uppercase();
    let pos = upper.find(&key_u)?;
    hay[pos + key.len()..].split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

fn extract_paren(hay: &str, key: &str) -> Option<String> {
    let upper = hay.to_ascii_uppercase();
    let key_u = key.to_ascii_uppercase();
    let pos = upper.find(&key_u)?;
    let after = hay[pos + key.len()..].trim_start();
    let inner = after.strip_prefix('(')?;
    let end = inner.find(')')?;
    Some(inner[..end].to_string())
}

pub(crate) fn extract_first_literal(hay: &str) -> Option<String> {
    let brace = hay.find('{')?;
    let close = hay[brace + 1..].find('}')?;
    let n: usize = hay[brace + 1..brace + 1 + close].parse().ok()?;
    let start = brace + 1 + close + 1;
    let bytes = hay.as_bytes().get(start..)?;
    let bytes = bytes.strip_prefix(b"\r\n").or_else(|| bytes.strip_prefix(b"\n")).unwrap_or(bytes);
    let take = n.min(bytes.len());
    Some(String::from_utf8_lossy(&bytes[..take]).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::CommandResult;

    #[test]
    fn parses_uid_fetch_with_header_literal() {
        let raw = CommandResult {
            ok: true,
            tagged: "OK".into(),
            untagged: vec![
                "* 1 FETCH (UID 42 FLAGS (\\Seen) RFC822.SIZE 120 BODY[HEADER] {87}\nFrom: Alice <alice@imyemail.test>\r\nSubject: Hi\r\nDate: Tue, 15 Sep 2026 10:00:00 +0800\r\n\r\n)".into(),
            ],
        };
        let items = parse_fetch_untagged(&raw);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].uid, 42);
        assert!(items[0].flags.seen);
        assert_eq!(items[0].headers.subject, "Hi");
        assert_eq!(items[0].headers.from[0].email, "alice@imyemail.test");
    }
}
