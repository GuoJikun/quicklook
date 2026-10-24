use quicklook_audio::parse_lrc;

fn write_lrc(name: &str, content: &str) -> String {
    let file = std::env::temp_dir().join(format!(
        "quicklook-audio-test-{name}-{}.lrc",
        std::process::id()
    ));
    std::fs::write(&file, content).unwrap();
    file.to_string_lossy().into_owned()
}

#[test]
fn parses_title_offset_and_timestamps() {
    let path = write_lrc(
        "basic",
        "[ti:测试歌曲]\n[offset:500]\n[00:01.50]第一句\n[01:02.25]第二句\n",
    );

    let lrc = parse_lrc(&path).unwrap();
    assert_eq!(lrc.title.as_deref(), Some("测试歌曲"));
    assert_eq!(lrc.offset, Some(500));
    assert_eq!(lrc.content.len(), 2);
    // 00:01.50 → 1500ms
    assert_eq!(lrc.content[0].timestamp, 1500);
    assert_eq!(lrc.content[0].text, "第一句");
    // 01:02.25 → 62250ms
    assert_eq!(lrc.content[1].timestamp, 62250);
    assert_eq!(lrc.content[1].text, "第二句");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn supports_integer_seconds_without_fraction() {
    let path = write_lrc("int-sec", "[00:02]整秒\n");

    let lrc = parse_lrc(&path).unwrap();
    assert_eq!(lrc.content.len(), 1);
    assert_eq!(lrc.content[0].timestamp, 2_000);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn sorts_lines_by_timestamp() {
    let path = write_lrc("sort", "[00:20.00]后\n[00:05.00]前\n[00:10.00]中\n");

    let lrc = parse_lrc(&path).unwrap();
    let texts: Vec<&str> = lrc.content.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(texts, vec!["前", "中", "后"]);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn skips_malformed_lines() {
    let path = write_lrc(
        "malformed",
        "[xx:yy]坏时间戳\n无时间轴文本\n[00:01.00]正常\n[bad offset\n",
    );

    let lrc = parse_lrc(&path).unwrap();
    assert_eq!(lrc.content.len(), 1);
    assert_eq!(lrc.content[0].text, "正常");
    // 非法 offset 不影响解析
    assert_eq!(lrc.offset, None);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn missing_file_returns_empty_result() {
    let path = std::env::temp_dir()
        .join("quicklook-audio-test-missing.lrc")
        .to_string_lossy()
        .into_owned();

    let lrc = parse_lrc(&path).unwrap();
    assert_eq!(lrc.title, None);
    assert_eq!(lrc.offset, None);
    assert!(lrc.content.is_empty());
}

#[test]
fn directory_path_returns_err() {
    let dir = std::env::temp_dir().join("quicklook-audio-test-dir");
    std::fs::create_dir_all(&dir).unwrap();

    assert!(parse_lrc(dir.to_str().unwrap()).is_err());

    let _ = std::fs::remove_dir_all(&dir);
}
