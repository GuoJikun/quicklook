use app_lib::utils::{detect_language_no_ext, get_file_info};

/// 在临时目录下创建指定文件名的测试文件，返回其绝对路径。
fn touch(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("quicklook-utils-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(name);
    std::fs::write(&file, b"x").unwrap();
    file.to_string_lossy().into_owned()
}

fn type_of(path: &str) -> Option<String> {
    get_file_info(path, &[], &[]).map(|f| f.get_file_type().to_string())
}

#[test]
fn maps_builtin_extensions_to_file_types() {
    let cases = [
        ("a.md", "Markdown"),
        ("b.markdown", "Markdown"),
        ("c.png", "Image"),
        ("d.PSD", "Image"),
        ("e.mp4", "Video"),
        ("f.mp3", "Audio"),
        ("g.epub", "Book"),
        ("h.ttf", "Font"),
        ("i.zip", "Archive"),
        ("j.rs", "Code"),
        ("k.json", "Code"),
        ("l.gltf", "Model3D"),
        ("m.docx", "Doc"),
        ("n.pdf", "Doc"),
        ("o.csv", "Doc"),
    ];

    for (name, expected) in cases {
        let path = touch(name);
        assert_eq!(
            type_of(&path).as_deref(),
            Some(expected),
            "扩展名 {} 的分类应为 {}",
            name,
            expected
        );
    }
}

#[test]
fn extension_is_normalized_to_lowercase() {
    let path = touch("Upper.PNG");
    let info = get_file_info(&path, &[], &[]).unwrap();
    assert_eq!(info.get_extension(), "png");
    assert_eq!(info.get_file_type(), "Image");
}

#[test]
fn custom_code_extension_maps_to_code() {
    let path = touch("script.myext");
    assert!(get_file_info(&path, &[], &[]).is_none());

    let custom = vec!["myext".to_string()];
    let info = get_file_info(&path, &custom, &[]).unwrap();
    assert_eq!(info.get_file_type(), "Code");
}

#[test]
fn custom_video_extension_maps_to_video() {
    let path = touch("clip.myvid");
    let custom = vec!["MYVID".to_string()];
    let info = get_file_info(&path, &[], &custom).unwrap();
    assert_eq!(info.get_file_type(), "Video");
}

#[test]
fn builtin_mapping_takes_precedence_over_custom() {
    // png 属内置 Image，不应被自定义扩展名覆盖为 Code
    let path = touch("pic.png");
    let custom = vec!["png".to_string()];
    let info = get_file_info(&path, &custom, &[]).unwrap();
    assert_eq!(info.get_file_type(), "Image");
}

#[test]
fn unknown_extension_returns_none() {
    let path = touch("data.unknownext");
    assert!(get_file_info(&path, &[], &[]).is_none());
}

#[test]
fn missing_file_returns_none() {
    let missing = std::env::temp_dir()
        .join("quicklook-utils-test-missing.md")
        .to_string_lossy()
        .into_owned();
    assert!(get_file_info(&missing, &[], &[]).is_none());
}

#[test]
fn directory_returns_none() {
    let dir = std::env::temp_dir().join(format!("quicklook-utils-test-dir-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    assert!(get_file_info(dir.to_str().unwrap(), &[], &[]).is_none());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn name_without_extension_uses_language_detection() {
    // README 无后缀 → detect_language_no_ext → "markdown" → Markdown
    let path = touch("README");
    let info = get_file_info(&path, &[], &[]).unwrap();
    assert_eq!(info.get_extension(), "markdown");
    assert_eq!(info.get_file_type(), "Markdown");
}

#[test]
fn detect_language_no_ext_known_names() {
    assert_eq!(detect_language_no_ext("README"), "markdown");
    assert_eq!(detect_language_no_ext("Makefile"), "makefile");
    assert_eq!(detect_language_no_ext("Dockerfile"), "docker");
    assert_eq!(detect_language_no_ext(".bashrc"), "bash");
    assert_eq!(detect_language_no_ext(".env"), "ini");
}

#[test]
fn detect_language_no_ext_falls_back_to_txt() {
    assert_eq!(detect_language_no_ext("随便什么"), "txt");
    assert_eq!(detect_language_no_ext(""), "txt");
}
