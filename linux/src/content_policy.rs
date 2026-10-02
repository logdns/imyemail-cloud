pub fn document(html: &str, allow_remote_images: bool) -> String {
    let images = if allow_remote_images { "https: data: cid:" } else { "data: cid:" };
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src {images}; base-uri 'none'; form-action 'none'; frame-src 'none'; object-src 'none'"><style>body{{margin:0;color:#202124;background:transparent;font-family:sans-serif;font-size:16px;line-height:1.65;overflow-wrap:anywhere}}img{{max-width:100%;height:auto}}img:not([src]),img[src=""],img[src^="http://"]{{display:none}}pre{{white-space:pre-wrap}}blockquote{{margin:16px 0;padding-left:16px;border-left:2px solid #8899bb}}</style></head><body>{html}</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_remote_images_by_default_and_allows_only_https_when_enabled() {
        let blocked = document(r#"<img src="https://images.test/a.png">"#, false);
        assert!(blocked.contains("img-src data: cid:"));
        assert!(!blocked.contains("img-src https:"));

        let allowed = document(r#"<img src="https://images.test/a.png">"#, true);
        assert!(allowed.contains("img-src https: data: cid:"));
        assert!(!allowed.contains("img-src http:"));
        assert!(allowed.contains("default-src 'none'"));
        assert!(allowed.contains("frame-src 'none'"));
    }
}
