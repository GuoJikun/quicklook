use app_lib::preview::route::WebRoute;
use app_lib::utils::File;

/// 通过 serde 构造测试用 File（字段与 utils::File 的 Serialize 保持一致）。
fn make_file() -> File {
    serde_json::from_str(
        r#"{
            "file_type": "Markdown",
            "path": "C:\\demo\\测试文档.md",
            "extension": "md",
            "size": 42,
            "last_modified": 100,
            "name": "测试文档.md"
        }"#,
    )
    .unwrap()
}

#[test]
fn maps_each_type_to_its_route() {
    let cases = [
        ("Markdown", "/preview/md"),
        ("Image", "/preview/image"),
        ("Audio", "/preview/audio"),
        ("Video", "/preview/video"),
        ("Font", "/preview/font"),
        ("Code", "/preview/code"),
        ("Book", "/preview/book"),
        ("Archive", "/preview/archive"),
        ("Doc", "/preview/document"),
        ("Model3D", "/preview/model"),
    ];

    for (type_str, expected) in cases {
        let route = WebRoute::get_route(type_str, make_file());
        assert_eq!(route.path, expected, "{} 应路由到 {}", type_str, expected);
    }
}

#[test]
fn unknown_type_falls_back_to_not_support() {
    for type_str in ["Unknown", "", "markdown", "IMAGE"] {
        let route = WebRoute::get_route(type_str, make_file());
        assert_eq!(
            route.path, "/preview/not-support",
            "类型 {} 应走兜底路由",
            type_str
        );
    }
}

#[test]
fn to_url_contains_route_and_query_params() {
    let route = WebRoute::get_route("Markdown", make_file());
    let url = route.to_url();

    assert!(url.starts_with("/preview/md?"), "实际: {}", url);
    assert!(url.contains("file_type=Markdown"));
    assert!(url.contains("extension=md"));
    assert!(url.contains("size=42"));
    assert!(url.contains("last_modified=100"));
    assert!(url.contains("name="));
}

#[test]
fn to_url_percent_encodes_special_characters() {
    let route = WebRoute::get_route("Markdown", make_file());
    let url = route.to_url();

    // path/name 中的中文与反斜杠应被百分号编码
    assert!(!url.contains("测试"), "中文应被编码: {}", url);
    assert!(url.contains("%"), "应存在百分号编码: {}", url);
}
