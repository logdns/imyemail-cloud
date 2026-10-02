//! Small, deterministic Markdown renderer used by the native composer.
//!
//! The source of truth remains the Markdown text (`body_text`).  This renderer
//! only emits a conservative HTML alternative: raw HTML is escaped, links are
//! limited to http/https/mailto, and images are represented as text.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Heading,
    Bold,
    Italic,
    Link,
    Bullet,
    Numbered,
    Quote,
    Code,
}

pub fn apply_format(
    source: &str,
    start: usize,
    end: usize,
    format: Format,
) -> (String, usize, usize) {
    let chars: Vec<char> = source.chars().collect();
    let start = start.min(chars.len());
    let end = end.max(start).min(chars.len());
    let selected: String = chars[start..end].iter().collect();
    let (replacement, cursor_start, cursor_len) = match format {
        Format::Heading | Format::Bullet | Format::Numbered | Format::Quote => {
            let prefix = match format {
                Format::Heading => "## ",
                Format::Numbered => "1. ",
                Format::Quote => "> ",
                _ => "- ",
            };
            let mut out = String::new();
            for (index, line) in selected.lines().enumerate() {
                if index > 0 {
                    out.push('\n');
                }
                if format == Format::Numbered {
                    out.push_str(&format!("{}. {}", index + 1, line));
                } else {
                    out.push_str(prefix);
                    out.push_str(line);
                }
            }
            if selected.ends_with('\n') {
                out.push('\n');
            }
            let len = out.chars().count();
            (out, start, len)
        }
        Format::Bold => wrapped("**", "**", &selected, start),
        Format::Italic => wrapped("*", "*", &selected, start),
        Format::Link => {
            let content = if selected.is_empty() { "链接" } else { selected.as_str() };
            let out = format!("[{content}](https://example.com)");
            let cursor = start + 1 + content.chars().count() + 2;
            (out, cursor, "https://example.com".chars().count())
        }
        Format::Code => {
            if selected.contains('\n') {
                let out = format!("\n```\n{selected}\n```\n");
                (out, start + 5, selected.chars().count())
            } else {
                let out = format!("`{selected}`");
                (out, start + 1, selected.chars().count())
            }
        }
    };
    let mut result: String = chars[..start].iter().collect();
    result.push_str(&replacement);
    result.extend(chars[end..].iter());
    (result, cursor_start, cursor_len)
}

fn wrapped(prefix: &str, suffix: &str, selected: &str, start: usize) -> (String, usize, usize) {
    let content = if selected.is_empty() { "文本" } else { selected };
    (format!("{prefix}{content}{suffix}"), start + prefix.chars().count(), content.chars().count())
}

pub fn render(source: &str) -> String {
    let mut out = String::from(
        "<div style=\"font-family:sans-serif;font-size:15px;line-height:1.65;overflow-wrap:anywhere\">",
    );
    let mut in_code = false;
    for raw in source.lines() {
        if raw.trim_start().starts_with("```") {
            if in_code {
                out.push_str("</code></pre>");
            } else {
                out.push_str("<pre style=\"white-space:pre-wrap;padding:12px;border:1px solid #999;border-radius:6px\"><code>");
            }
            in_code = !in_code;
            continue;
        }
        if in_code {
            out.push_str(&escape(raw));
            out.push('\n');
            continue;
        }
        let trimmed = raw.trim_start();
        let (tag, content) = if let Some(value) = trimmed.strip_prefix("### ") {
            ("h3", value)
        } else if let Some(value) = trimmed.strip_prefix("## ") {
            ("h2", value)
        } else if let Some(value) = trimmed.strip_prefix("# ") {
            ("h1", value)
        } else if let Some(value) = trimmed.strip_prefix("> ") {
            ("blockquote", value)
        } else {
            ("p", raw)
        };
        out.push('<');
        out.push_str(tag);
        out.push('>');
        out.push_str(&inline(content));
        out.push_str("</");
        out.push_str(tag);
        out.push('>');
    }
    if in_code {
        out.push_str("</code></pre>");
    }
    out.push_str("</div>");
    out
}

fn inline(value: &str) -> String {
    let escaped = escape(value);
    let mut out = String::new();
    let mut rest = escaped.as_str();
    while let Some(start) = rest.find("**") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("**") else {
            out.push_str(&rest[start..]);
            return out;
        };
        out.push_str("<strong>");
        out.push_str(&after[..end]);
        out.push_str("</strong>");
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out.replace("  ", "<br>")
}

fn escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_safe_markdown_without_raw_html() {
        let html = render("# Hello\n\n**world**\n<script>alert(1)</script>");
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>world</strong>"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn formatting_preserves_unicode_selection() {
        let (value, start, len) = apply_format("你好", 0, 2, Format::Bold);
        assert_eq!(value, "**你好**");
        assert_eq!((start, len), (2, 2));
    }
}
