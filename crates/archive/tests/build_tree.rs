use quicklook_archive::{Extract, normalize_entry_name};

fn entry(name: &str, size: u64, mtime: &str, dir: bool) -> Extract {
    Extract::new(name.to_string(), size, mtime.to_string(), dir)
}

fn find<'a>(nodes: &'a [Extract], path: &str) -> Option<&'a Extract> {
    nodes.iter().find(|n| n.name == path)
}

fn find_in_children<'a>(parent: &'a Extract, path: &str) -> Option<&'a Extract> {
    parent
        .children
        .as_ref()
        .and_then(|kids| kids.iter().find(|n| n.name == path))
}

#[test]
fn normalize_backslashes_and_duplicate_slashes() {
    assert_eq!(normalize_entry_name("a\\b\\c.txt"), "a/b/c.txt");
    assert_eq!(normalize_entry_name("a//b///c.txt"), "a/b/c.txt");
    assert_eq!(normalize_entry_name("a/b/c.txt"), "a/b/c.txt");
}

#[test]
fn build_tree_synthesizes_missing_intermediate_dirs() {
    // 仅有深层文件、无任何目录条目（常见于 JAR/部分 ZIP）
    let tree = Extract::build_tree(vec![entry("a/b/c.txt", 10, "2024-01-01 00:00:00", false)]);

    assert_eq!(tree.len(), 1);
    let a = &tree[0];
    assert_eq!(a.name, "a");
    assert!(a.dir);
    assert!(a.last_modified.is_empty());

    let b = find_in_children(a, "a/b").expect("should synthesize a/b");
    assert!(b.dir);

    let c = find_in_children(b, "a/b/c.txt").expect("should keep file");
    assert_eq!(c.size, 10);
    assert!(!c.dir);
}

#[test]
fn build_tree_keeps_explicit_dir_metadata() {
    let tree = Extract::build_tree(vec![
        entry("a/", 0, "2020-06-15 12:00:00", true),
        entry("a/b.txt", 3, "2021-01-01 00:00:00", false),
    ]);

    let a = find(&tree, "a").expect("dir a");
    assert!(a.dir);
    assert_eq!(a.last_modified, "2020-06-15 12:00:00");
    let _ = find_in_children(a, "a/b.txt").expect("child file");
}

#[test]
fn build_tree_merges_dir_metadata_after_synthesis() {
    // 先出现子文件、后出现目录条目时，目录元数据仍应保留
    let tree = Extract::build_tree(vec![
        entry("a/b/c.txt", 1, "", false),
        entry("a/b/", 0, "2022-03-04 05:06:07", true),
    ]);

    let a = find(&tree, "a").expect("a");
    let b = find_in_children(a, "a/b").expect("a/b");
    assert_eq!(b.last_modified, "2022-03-04 05:06:07");
}

#[test]
fn build_tree_normalizes_backslash_paths() {
    let tree = Extract::build_tree(vec![entry("dir\\sub\\file.bin", 5, "", false)]);
    assert_eq!(tree[0].name, "dir");
    let sub = find_in_children(&tree[0], "dir/sub").expect("dir/sub");
    let _ = find_in_children(sub, "dir/sub/file.bin").expect("file");
}

#[test]
fn build_tree_root_files() {
    let tree = Extract::build_tree(vec![
        entry("readme.txt", 1, "", false),
        entry("z.txt", 2, "", false),
    ]);
    assert_eq!(tree.len(), 2);
    assert_eq!(tree[0].name, "readme.txt");
    assert_eq!(tree[1].name, "z.txt");
}

#[test]
fn build_tree_children_sorted_by_name() {
    let tree = Extract::build_tree(vec![
        entry("a/z.txt", 1, "", false),
        entry("a/a.txt", 1, "", false),
        entry("a/m/", 0, "", true),
    ]);
    let a = find(&tree, "a").expect("a");
    let names: Vec<_> = a
        .children
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, vec!["a/a.txt", "a/m", "a/z.txt"]);
}
