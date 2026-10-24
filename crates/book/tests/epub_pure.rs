use quicklook_book::epub::{
    ResourceMap, guess_mime_type, has_prefix_boundary, has_suffix_boundary, inline_css_urls,
    is_attr_boundary, replace_attr_value, resolve_path, to_data_uri,
};

// ---------- guess_mime_type ----------

#[test]
fn guess_mime_type_maps_common_image_formats() {
    assert_eq!(guess_mime_type("cover.png"), "image/png");
    assert_eq!(guess_mime_type("photo.JPG"), "image/jpeg");
    assert_eq!(guess_mime_type("photo.jpeg"), "image/jpeg");
    assert_eq!(guess_mime_type("anim.gif"), "image/gif");
    assert_eq!(guess_mime_type("icon.svg"), "image/svg+xml");
    assert_eq!(guess_mime_type("icon.svgz"), "image/svg+xml");
    assert_eq!(guess_mime_type("pic.webp"), "image/webp");
    assert_eq!(guess_mime_type("pic.bmp"), "image/bmp");
}

#[test]
fn guess_mime_type_maps_font_and_css() {
    assert_eq!(guess_mime_type("style.css"), "text/css");
    assert_eq!(guess_mime_type("font.ttf"), "font/ttf");
    assert_eq!(guess_mime_type("font.otf"), "font/otf");
    assert_eq!(guess_mime_type("font.woff"), "font/woff");
    assert_eq!(guess_mime_type("font.woff2"), "font/woff2");
}

#[test]
fn guess_mime_type_falls_back_to_octet_stream() {
    assert_eq!(guess_mime_type("data.bin"), "application/octet-stream");
    assert_eq!(guess_mime_type("unknown.xyz"), "application/octet-stream");
    assert_eq!(guess_mime_type(""), "application/octet-stream");
}

#[test]
fn guess_mime_type_is_case_insensitive() {
    assert_eq!(guess_mime_type("Cover.PNG"), "image/png");
    assert_eq!(guess_mime_type("Style.CSS"), "text/css");
}

// ---------- has_suffix_boundary ----------

#[test]
fn suffix_boundary_rejects_partial_name_match() {
    // ch10.xhtml 不应匹配 ch1.xhtml（ch 后紧跟数字 10，非路径分隔符）
    assert!(!has_suffix_boundary("Text/ch10.xhtml", "ch1.xhtml"));
}

#[test]
fn suffix_boundary_accepts_path_separator_boundary() {
    assert!(has_suffix_boundary("Text/ch1.xhtml", "ch1.xhtml"));
    assert!(has_suffix_boundary("OEBPS/ch1.xhtml", "ch1.xhtml"));
}

#[test]
fn suffix_boundary_accepts_exact_match() {
    assert!(has_suffix_boundary("ch1.xhtml", "ch1.xhtml"));
}

#[test]
fn suffix_boundary_rejects_non_matching_suffix() {
    assert!(!has_suffix_boundary("Text/ch2.xhtml", "ch1.xhtml"));
}

#[test]
fn suffix_boundary_empty_needle_matches() {
    assert!(has_suffix_boundary("anything", ""));
}

// ---------- has_prefix_boundary ----------

#[test]
fn prefix_boundary_rejects_partial_name_match() {
    // ch10 不应匹配 ch1（ch1 后紧跟数字 0）
    assert!(!has_prefix_boundary("ch10.xhtml", "ch1"));
}

#[test]
fn prefix_boundary_accepts_dot_boundary() {
    assert!(has_prefix_boundary("ch1.xhtml", "ch1"));
}

#[test]
fn prefix_boundary_accepts_slash_boundary() {
    assert!(has_prefix_boundary("ch1/sub.xhtml", "ch1"));
}

#[test]
fn prefix_boundary_accepts_exact_match() {
    assert!(has_prefix_boundary("ch1", "ch1"));
}

#[test]
fn prefix_boundary_rejects_embedded_match() {
    assert!(!has_prefix_boundary("OEBPS/ch1.xhtml", "ch1"));
}

// ---------- is_attr_boundary ----------

#[test]
fn attr_boundary_accepts_start_of_string() {
    assert!(is_attr_boundary("src=\"a.png\"", 0));
}

#[test]
fn attr_boundary_accepts_after_whitespace() {
    // " src=..." — src 起始于位置 1，前一个字符是空格
    assert!(is_attr_boundary(" src=\"a.png\"", 1));
}

#[test]
fn attr_boundary_rejects_after_alphanumeric() {
    // data-src= 中的 src= 前面是 '-'，不是合法边界
    assert!(!is_attr_boundary("data-src=\"a.png\"", 5));
}

#[test]
fn attr_boundary_accepts_after_gt_or_slash() {
    // 前一个字符是 '>' 或 '/' 时视为合法边界
    assert!(is_attr_boundary(">src=\"a.png\"", 1));
    assert!(is_attr_boundary("/src=\"a.png\"", 1));
}

// ---------- replace_attr_value ----------

#[test]
fn replace_double_quoted_attr() {
    let result = replace_attr_value(r#" src="old.png" alt="t""#, "src", "old.png", "new.png");
    assert_eq!(result, r#" src="new.png" alt="t""#);
}

#[test]
fn replace_single_quoted_attr() {
    let result = replace_attr_value(" src='old.png' alt='t'", "src", "old.png", "new.png");
    assert_eq!(result, " src='new.png' alt='t'");
}

#[test]
fn replace_returns_original_when_not_found() {
    let input = r#" src="other.png""#;
    let result = replace_attr_value(input, "src", "old.png", "new.png");
    assert_eq!(result, input);
}

#[test]
fn replace_only_first_occurrence() {
    let input = r#" src="a.png" data-src="a.png""#;
    let result = replace_attr_value(input, "src", "a.png", "b.png");
    assert!(result.contains(r#"src="b.png""#));
    assert!(result.contains(r#"data-src="a.png""#));
}

// ---------- to_data_uri ----------

#[test]
fn to_data_uri_encodes_bytes_to_base64() {
    // "hello" 的 base64 是 "aGVsbG8="
    assert_eq!(
        to_data_uri("text/plain", b"hello"),
        "data:text/plain;base64,aGVsbG8="
    );
}

#[test]
fn to_data_uri_handles_empty_data() {
    assert_eq!(to_data_uri("image/png", b""), "data:image/png;base64,");
}

// ---------- resolve_path ----------

#[test]
fn resolve_relative_to_chapter_dir() {
    assert_eq!(resolve_path("Text", "image.png"), "Text/image.png");
    assert_eq!(resolve_path("Text", "./image.png"), "Text/image.png");
}

#[test]
fn resolve_parent_directory_reference() {
    assert_eq!(
        resolve_path("Text", "../Images/cover.png"),
        "Images/cover.png"
    );
    assert_eq!(
        resolve_path("Text/Section", "../Images/cover.png"),
        "Text/Images/cover.png"
    );
}

#[test]
fn resolve_root_absolute_path() {
    assert_eq!(
        resolve_path("Text", "/Images/cover.png"),
        "Images/cover.png"
    );
}

#[test]
fn resolve_preserves_external_uris() {
    assert_eq!(
        resolve_path("Text", "https://example.com/a.png"),
        "https://example.com/a.png"
    );
    assert_eq!(
        resolve_path("Text", "data:image/png;base64,abc"),
        "data:image/png;base64,abc"
    );
    assert_eq!(
        resolve_path("Text", "mailto:test@example.com"),
        "mailto:test@example.com"
    );
}

#[test]
fn resolve_empty_returns_base_dir() {
    assert_eq!(resolve_path("Text", ""), "Text");
    assert_eq!(resolve_path("Text", "  "), "Text");
}

// ---------- inline_css_urls ----------

fn make_resources(entries: &[(&str, &str, &[u8])]) -> ResourceMap {
    entries
        .iter()
        .map(|(path, mime, data)| (path.to_string(), (mime.to_string(), data.to_vec())))
        .collect()
}

#[test]
fn inline_css_replaces_double_quoted_url() {
    let resources = make_resources(&[("Text/a.png", "image/png", b"\x89PNG")]);
    let css = r#"body { background: url("a.png"); }"#;
    let result = inline_css_urls(css, "Text", &resources);
    assert!(
        result.contains("data:image/png;base64,"),
        "实际: {}",
        result
    );
    assert!(!result.contains("a.png"), "应替换原路径: {}", result);
}

#[test]
fn inline_css_replaces_single_quoted_url() {
    let resources = make_resources(&[("Text/a.png", "image/png", b"\x89PNG")]);
    let css = "body { background: url('a.png'); }";
    let result = inline_css_urls(css, "Text", &resources);
    assert!(
        result.contains("data:image/png;base64,"),
        "实际: {}",
        result
    );
}

#[test]
fn inline_css_replaces_unquoted_url() {
    let resources = make_resources(&[("Text/a.png", "image/png", b"\x89PNG")]);
    let css = "body { background: url(a.png); }";
    let result = inline_css_urls(css, "Text", &resources);
    assert!(
        result.contains("data:image/png;base64,"),
        "实际: {}",
        result
    );
}

#[test]
fn inline_css_skips_data_uri() {
    let resources = make_resources(&[]);
    let css = r#"body { background: url("data:image/png;base64,abc"); }"#;
    let result = inline_css_urls(css, "Text", &resources);
    assert_eq!(result, css);
}

#[test]
fn inline_css_skips_empty_url() {
    let resources = make_resources(&[]);
    let css = r#"body { background: url(""); }"#;
    let result = inline_css_urls(css, "Text", &resources);
    assert_eq!(result, css);
}

#[test]
fn inline_css_keeps_unknown_resource() {
    let resources = make_resources(&[]);
    let css = r#"body { background: url("missing.png"); }"#;
    let result = inline_css_urls(css, "Text", &resources);
    assert_eq!(result, css);
}

#[test]
fn inline_css_skips_octet_stream_mime() {
    let resources = make_resources(&[("Text/a.bin", "application/octet-stream", b"data")]);
    let css = r#"body { background: url("a.bin"); }"#;
    let result = inline_css_urls(css, "Text", &resources);
    assert_eq!(result, css);
}

#[test]
fn inline_css_handles_multiple_urls() {
    let resources = make_resources(&[
        ("Text/a.png", "image/png", b"AAA"),
        ("Text/b.png", "image/png", b"BBB"),
    ]);
    let css = r#"a { background: url("a.png"); } b { background: url("b.png"); }"#;
    let result = inline_css_urls(css, "Text", &resources);
    assert!(
        result.contains("data:image/png;base64,QUFB"),
        "实际: {}",
        result
    );
    assert!(
        result.contains("data:image/png;base64,QkJC"),
        "实际: {}",
        result
    );
}
