use quicklook_cache::{CacheDir, file_key};
use std::path::PathBuf;

fn unique_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "quicklook-cache-test-{name}-{}",
        std::process::id()
    ))
}

// ---------- CacheDir ----------

#[test]
fn clear_removes_files_and_dirs() {
    let root = unique_root("clear");
    let cache = CacheDir::with_root(&root, "images");
    cache.ensure().unwrap();

    std::fs::write(cache.file_path("a.png"), b"a").unwrap();
    std::fs::create_dir_all(cache.path().join("sub")).unwrap();
    std::fs::write(cache.path().join("sub/b.bin"), b"b").unwrap();

    assert_eq!(cache.clear().unwrap(), 2);
    assert!(cache.path().exists());
    assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn clear_dirs_with_prefix_only_removes_matching_dirs() {
    let root = unique_root("prefix");
    let cache = CacheDir::with_root(&root, "videos");
    cache.ensure().unwrap();

    std::fs::create_dir_all(cache.path().join("quicklook_hls_a")).unwrap();
    std::fs::create_dir_all(cache.path().join("quicklook_hls_b")).unwrap();
    std::fs::create_dir_all(cache.path().join("keep")).unwrap();
    std::fs::write(cache.file_path("note.txt"), b"x").unwrap();

    assert_eq!(cache.clear_dirs_with_prefix("quicklook_hls_").unwrap(), 2);
    assert!(!cache.path().join("quicklook_hls_a").exists());
    assert!(!cache.path().join("quicklook_hls_b").exists());
    assert!(cache.path().join("keep").exists());
    assert!(cache.file_path("note.txt").exists());

    let _ = std::fs::remove_dir_all(&root);
}

// ---------- file_key ----------

#[test]
fn file_key_is_deterministic() {
    let root = unique_root("det");
    let cache = CacheDir::with_root(&root, "files");
    cache.ensure().unwrap();
    let file = cache.file_path("stable.txt");
    std::fs::write(&file, b"same content").unwrap();

    let path = file.to_string_lossy();
    assert_eq!(file_key(path.as_ref()), file_key(path.as_ref()));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn file_key_changes_with_file_content() {
    let root = unique_root("key");
    let cache = CacheDir::with_root(&root, "files");
    cache.ensure().unwrap();
    let file = cache.file_path("source.txt");

    std::fs::write(&file, b"a").unwrap();
    let first = file_key(file.to_string_lossy().as_ref());
    std::fs::write(&file, b"changed content").unwrap();
    let second = file_key(file.to_string_lossy().as_ref());

    assert_ne!(first, second);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn file_key_differs_for_different_paths_with_same_content() {
    let root = unique_root("path");
    let cache = CacheDir::with_root(&root, "files");
    cache.ensure().unwrap();
    let a = cache.file_path("a.txt");
    let b = cache.file_path("b.txt");
    std::fs::write(&a, b"same").unwrap();
    std::fs::write(&b, b"same").unwrap();

    assert_ne!(
        file_key(a.to_string_lossy().as_ref()),
        file_key(b.to_string_lossy().as_ref())
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn file_key_does_not_panic_for_missing_file() {
    let missing = std::env::temp_dir().join("quicklook-cache-test-definitely-missing.txt");
    assert!(!missing.exists());

    // 不存在的文件退化为仅按路径哈希，且结果稳定
    let path = missing.to_string_lossy();
    assert_eq!(file_key(path.as_ref()), file_key(path.as_ref()));
}
