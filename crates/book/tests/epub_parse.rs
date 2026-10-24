use quicklook_book::epub::{
    extract_attr_value, match_spine_href, parse_css_url_expr, resolve_path,
};

// ---------- resolve_path ----------

#[test]
fn resolves_relative_paths_against_current_chapter_dir() {
    assert_eq!(
        resolve_path("Text", "chapter1.xhtml"),
        "Text/chapter1.xhtml"
    );
    assert_eq!(
        resolve_path("Text", "./chapter1.xhtml"),
        "Text/chapter1.xhtml"
    );
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
fn preserves_external_and_root_paths() {
    assert_eq!(
        resolve_path("Text", "https://example.com/a.png"),
        "https://example.com/a.png"
    );
    assert_eq!(
        resolve_path("Text", "/Images/cover.png"),
        "Images/cover.png"
    );
    assert_eq!(
        resolve_path("Text", "data:image/png;base64,abc"),
        "data:image/png;base64,abc"
    );
}

// ---------- extract_attr_value ----------

#[test]
fn extracts_attr_value_with_quote_styles() {
    assert_eq!(
        extract_attr_value(r#" src="a.png""#, "src").as_deref(),
        Some("a.png")
    );
    assert_eq!(
        extract_attr_value(" src='a.png'", "src").as_deref(),
        Some("a.png")
    );
    assert_eq!(
        extract_attr_value(" src=a.png ", "src").as_deref(),
        Some("a.png")
    );
}

#[test]
fn does_not_match_partial_attr_names() {
    assert_eq!(extract_attr_value(r#" data-src="a.png""#, "src"), None);
    assert_eq!(extract_attr_value(r#" xlink:href="a.png""#, "href"), None);
    assert_eq!(extract_attr_value(r#" data-href="a.png""#, "href"), None);
}

#[test]
fn finds_attr_after_other_attributes() {
    assert_eq!(
        extract_attr_value(r#" class="x" src="a.png" alt="t""#, "src").as_deref(),
        Some("a.png")
    );
    assert_eq!(
        extract_attr_value(r#" xlink:href="a.png" href="b.png""#, "href").as_deref(),
        Some("b.png")
    );
}

// ---------- match_spine_href ----------

#[test]
fn matches_spine_href_relative_and_fragment() {
    let spine = vec![
        "Text/ch1.xhtml".to_string(),
        "Text/ch2.xhtml".to_string(),
        "OEBPS/appendix.xhtml".to_string(),
    ];
    assert_eq!(match_spine_href(&spine, 0, "ch2.xhtml"), Some((1, None)));
    assert_eq!(
        match_spine_href(&spine, 0, "ch2.xhtml#sec"),
        Some((1, Some("sec".to_string())))
    );
    assert_eq!(
        match_spine_href(&spine, 1, "appendix.xhtml"),
        Some((2, None))
    );
    assert_eq!(
        match_spine_href(&spine, 0, "/OEBPS/appendix.xhtml"),
        Some((2, None))
    );
}

// ---------- parse_css_url_expr ----------

#[test]
fn parse_css_url_handles_quoted_and_unquoted() {
    // `"a.png")` 长度 8；`'a.png')` 长度 8；`a.png)` 长度 6
    assert_eq!(parse_css_url_expr(r#""a.png") rest"#), Some(("a.png", 8)));
    assert_eq!(parse_css_url_expr("'a.png') rest"), Some(("a.png", 8)));
    assert_eq!(parse_css_url_expr("a.png) rest"), Some(("a.png", 6)));
}
