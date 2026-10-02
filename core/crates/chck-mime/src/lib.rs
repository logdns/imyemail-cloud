//! MIME 解析：multipart / text / html。S3 覆盖常见邮件。

use chck_types::EngineError;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedAttachment {
    pub name: String,
    pub mime: String,
    pub cid: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedMessage {
    pub subject: String,
    pub message_id: Option<String>,
    pub text: String,
    pub html: Option<String>,
    pub attachments: Vec<ParsedAttachment>,
}

/// RFC 2047 encoded words, folded headers and legacy charsets.
#[must_use]
pub fn decode_header(value: &str) -> String {
    let raw = format!("Subject: {value}");
    mailparse::parse_header(raw.as_bytes())
        .map(|(h, _)| h.get_value())
        .unwrap_or_else(|_| value.to_owned())
}

pub fn parse(raw: &[u8]) -> Result<ParsedMessage, EngineError> {
    parse_str(&String::from_utf8_lossy(raw))
}

pub fn parse_str(raw: &str) -> Result<ParsedMessage, EngineError> {
    let (headers, body) = split_headers_body(raw);
    let mut parsed = ParsedMessage {
        subject: decode_header(&header_value(&headers, "subject").unwrap_or_default()),
        message_id: header_value(&headers, "message-id"),
        ..ParsedMessage::default()
    };
    decode_entity(&mut parsed, &headers, body);
    if parsed.text.is_empty()
        && let Some(html) = &parsed.html
    {
        parsed.text = strip_tags(html);
    }
    Ok(parsed)
}

fn decode_entity(parsed: &mut ParsedMessage, headers: &str, body: &str) {
    let ct = header_value(headers, "content-type").unwrap_or_else(|| "text/plain".into());
    if let Some(boundary) = mime_param(&ct, "boundary") {
        for part in split_multipart(body, &boundary) {
            let (ph, pb) = split_headers_body(&part);
            decode_entity(parsed, &ph, pb);
        }
        return;
    }
    let enc = header_value(headers, "content-transfer-encoding");
    let lower = ct.to_ascii_lowercase();
    let disp = header_value(headers, "content-disposition").unwrap_or_default();
    let disp_l = disp.to_ascii_lowercase();
    let is_attach = disp_l.contains("attachment")
        || (!lower.contains("text/")
            && !lower.contains("multipart")
            && !lower.starts_with("message/"));
    if is_attach || lower.contains("image/") {
        let name = mime_param(&disp, "filename")
            .or_else(|| mime_param(&ct, "name"))
            .unwrap_or_else(|| "attachment".into());
        parsed.attachments.push(ParsedAttachment {
            name,
            mime: lower.split(';').next().unwrap_or("application/octet-stream").trim().into(),
            cid: header_value(headers, "content-id")
                .map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string()),
            data: decode_bytes(body, enc.as_deref()),
        });
        return;
    }
    let decoded = decode_body(body, enc.as_deref());
    if lower.contains("text/html") {
        if parsed.html.is_none() {
            parsed.html = Some(decoded);
        }
    } else if lower.contains("text/plain") && parsed.text.is_empty() {
        parsed.text = decoded;
    }
}

fn split_headers_body(raw: &str) -> (String, &str) {
    if let Some(idx) = raw.find("\r\n\r\n") {
        (raw[..idx].to_string(), &raw[idx + 4..])
    } else if let Some(idx) = raw.find("\n\n") {
        (raw[..idx].to_string(), &raw[idx + 2..])
    } else {
        (raw.to_string(), "")
    }
}

fn header_value(headers: &str, name: &str) -> Option<String> {
    let mut current: Option<(String, String)> = None;
    let mut found = None;
    for line in headers.replace("\r\n", "\n").lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some((_, ref mut v)) = current {
                v.push(' ');
                v.push_str(line.trim());
            }
            continue;
        }
        if let Some((n, v)) = current.take()
            && n == name
        {
            found = Some(v);
        }
        if let Some((n, v)) = line.split_once(':') {
            current = Some((n.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    if let Some((n, v)) = current
        && n == name
    {
        found = Some(v);
    }
    found
}

fn mime_param(ct: &str, name: &str) -> Option<String> {
    for part in ct.split(';').skip(1) {
        if let Some((k, v)) = part.split_once('=')
            && k.trim().eq_ignore_ascii_case(name)
        {
            return Some(v.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn split_multipart(body: &str, boundary: &str) -> Vec<String> {
    let delim = format!("--{boundary}");
    body.split(&delim)
        .skip(1)
        .filter(|p| !p.trim_start().starts_with("--"))
        .map(|p| p.trim_start_matches("\r\n").trim_start_matches('\n').to_string())
        .collect()
}

fn decode_body(body: &str, enc: Option<&str>) -> String {
    let enc = enc.unwrap_or("").to_ascii_lowercase();
    if enc.contains("quoted-printable") {
        return decode_qp(body);
    }
    if enc.contains("base64") {
        return String::from_utf8_lossy(&decode_b64(body)).into_owned();
    }
    body.trim_end().to_string()
}

fn decode_bytes(body: &str, enc: Option<&str>) -> Vec<u8> {
    let enc = enc.unwrap_or("").to_ascii_lowercase();
    if enc.contains("base64") {
        return decode_b64(body);
    }
    if enc.contains("quoted-printable") {
        return decode_qp(body).into_bytes();
    }
    body.as_bytes().to_vec()
}

fn decode_b64(input: &str) -> Vec<u8> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut table = [0xffu8; 256];
    for (i, b) in T.iter().enumerate() {
        table[*b as usize] = i as u8;
    }
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut n = 0u32;
    for b in input.bytes() {
        if b == b'=' || b.is_ascii_whitespace() {
            continue;
        }
        let v = table[b as usize];
        if v == 0xff {
            continue;
        }
        acc = (acc << 6) | u32::from(v);
        n += 1;
        if n == 4 {
            out.push((acc >> 16) as u8);
            out.push((acc >> 8) as u8);
            out.push(acc as u8);
            acc = 0;
            n = 0;
        }
    }
    if n == 3 {
        acc <<= 6;
        out.push((acc >> 16) as u8);
        out.push((acc >> 8) as u8);
    } else if n == 2 {
        acc <<= 12;
        out.push((acc >> 16) as u8);
    }
    out
}

fn decode_qp(input: &str) -> String {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            if i + 1 < bytes.len() && (bytes[i + 1] == b'\n' || bytes[i + 1] == b'\r') {
                i += 1;
                while i < bytes.len() && (bytes[i] == b'\r' || bytes[i] == b'\n') {
                    i += 1;
                }
                continue;
            }
            if i + 2 < bytes.len()
                && let Ok(v) =
                    u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
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
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain() {
        let raw = "Subject: Hi\r\nContent-Type: text/plain\r\n\r\nHello world\n";
        let p = parse_str(raw).unwrap();
        assert_eq!(p.text.trim(), "Hello world");
    }

    #[test]
    fn parses_multipart_html() {
        let raw = "Content-Type: multipart/alternative; boundary=b\r\n\r\n\
--b\r\nContent-Type: text/plain\r\n\r\nplain\r\n\
--b\r\nContent-Type: text/html\r\n\r\n<p>html</p>\r\n\
--b--\r\n";
        let p = parse_str(raw).unwrap();
        assert_eq!(p.text.trim(), "plain");
        assert!(p.html.as_deref().unwrap().contains("<p>html</p>"));
    }

    #[test]
    fn parses_base64_attachment() {
        let raw = "Content-Type: multipart/mixed; boundary=b\r\n\r\n\
--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n\
--b\r\nContent-Type: application/pdf; name=\"a.pdf\"\r\n\
Content-Disposition: attachment; filename=\"a.pdf\"\r\n\
Content-Transfer-Encoding: base64\r\n\r\nAQID\r\n\
--b--\r\n";
        let p = parse_str(raw).unwrap();
        assert_eq!(p.text.trim(), "hello");
        assert_eq!(p.attachments.len(), 1);
        assert_eq!(p.attachments[0].name, "a.pdf");
        assert_eq!(p.attachments[0].data, vec![1, 2, 3]);
    }
}
