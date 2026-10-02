//! HTML 净化：白名单标签、剥离脚本并标记可选的 HTTPS 图片。六端统一。

use ammonia::{Builder, UrlRelative};
use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizeResult {
    pub html: String,
    pub remote_blocked: u32,
}

/// 净化邮件 HTML。HTTPS 图片地址保留给渲染层的可选加载策略；
/// HTTP、相对路径和其他外部图片始终剥离。
#[must_use]
pub fn sanitize_html(html: &str) -> SanitizeResult {
    let blocked = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&blocked);
    let mut builder = Builder::default();
    builder
        .tags(safe_tags())
        .url_schemes(HashSet::from(["https", "http", "mailto", "cid"]))
        .url_relative(UrlRelative::Deny)
        .link_rel(Some("noopener noreferrer"))
        .strip_comments(true)
        .attribute_filter(move |element, attribute, value| {
            if element == "img" && attribute == "src" {
                // Embedded MIME parts may load automatically. Keep HTTPS addresses
                // so a renderer can opt in under CSP, but never retain clear-text,
                // relative, file, data or other attacker-controlled schemes.
                let value = value.trim();
                if value.get(..4).is_some_and(|scheme| scheme.eq_ignore_ascii_case("cid:")) {
                    return Some(value.into());
                }
                counter.fetch_add(1, Ordering::Relaxed);
                if value.get(..8).is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://")) {
                    return Some(format!("https://{}", &value[8..]).into());
                }
                return None;
            }
            Some(value.into())
        });
    let html = builder.clean(html).to_string();
    let remote_blocked = blocked.load(Ordering::Relaxed);
    SanitizeResult { html, remote_blocked }
}

fn safe_tags() -> HashSet<&'static str> {
    HashSet::from([
        "a",
        "p",
        "br",
        "div",
        "span",
        "b",
        "i",
        "u",
        "em",
        "strong",
        "blockquote",
        "pre",
        "code",
        "ul",
        "ol",
        "li",
        "table",
        "thead",
        "tbody",
        "tr",
        "td",
        "th",
        "img",
        "h1",
        "h2",
        "h3",
        "h4",
        "hr",
        "sup",
        "sub",
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script_and_javascript_uri() {
        let r = sanitize_html(
            r#"<p>hi</p><script>alert(1)</script><a href="javascript:alert(1)">x</a>"#,
        );
        assert!(!r.html.contains("script"));
        assert!(!r.html.contains("javascript:"));
        assert!(r.html.contains("hi"));
    }

    #[test]
    fn preserves_https_images_for_opt_in_rendering() {
        let r = sanitize_html(r#"<p>ok</p><img src="https://tracker.example/pixel.gif">"#);
        assert_eq!(r.remote_blocked, 1);
        assert!(r.html.contains("src=\"https://tracker.example/pixel.gif\""));
    }

    #[test]
    fn blocks_obfuscated_remote_images_without_removing_links() {
        for source in [
            "HTTPS://tracker.example/pixel",
            " HTTPs://tracker.example/pixel ",
            "&#104;ttps://tracker.example/pixel",
        ] {
            let result = sanitize_html(&format!(
                r#"<IMG SRC='{source}'><a href="https://example.com">link</a>"#
            ));
            assert!(
                result.html.contains("src=\"https://tracker.example/pixel\""),
                "{}",
                result.html
            );
            assert!(result.html.contains("href=\"https://example.com\""));
            assert_eq!(result.remote_blocked, 1, "{source}");
        }
        let relative = sanitize_html(r#"<img src="//tracker.example/pixel">"#);
        assert!(!relative.html.contains("src="));

        for source in
            ["http://tracker.example/pixel", "file:///tmp/secret", "data:image/png;base64,AAAA"]
        {
            let result = sanitize_html(&format!(r#"<img src='{source}'>"#));
            assert!(!result.html.contains("src="), "{}", result.html);
        }
    }

    #[test]
    fn keeps_cid_images() {
        let r = sanitize_html(r#"<img src="cid:part1">"#);
        assert!(r.html.contains("cid:part1"));
        assert_eq!(r.remote_blocked, 0);
    }
}
