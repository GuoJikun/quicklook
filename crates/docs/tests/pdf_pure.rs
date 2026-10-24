use quicklook_docs::pdf::{Lru, normalize_rotation};

// ---------- normalize_rotation ----------

#[test]
fn normalize_rotation_passes_valid_values() {
    assert_eq!(normalize_rotation(0), 0);
    assert_eq!(normalize_rotation(90), 90);
    assert_eq!(normalize_rotation(180), 180);
    assert_eq!(normalize_rotation(270), 270);
}

#[test]
fn normalize_rotation_wraps_around_360() {
    assert_eq!(normalize_rotation(360), 0);
    assert_eq!(normalize_rotation(450), 90);
    assert_eq!(normalize_rotation(540), 180);
    assert_eq!(normalize_rotation(630), 270);
}

#[test]
fn normalize_rotation_falls_back_to_zero_for_invalid() {
    assert_eq!(normalize_rotation(45), 0);
    assert_eq!(normalize_rotation(1), 0);
    assert_eq!(normalize_rotation(359), 0);
    assert_eq!(normalize_rotation(120), 0);
}

#[test]
fn normalize_rotation_handles_large_values() {
    assert_eq!(normalize_rotation(3600), 0);
    assert_eq!(normalize_rotation(3690), 90);
}

// ---------- Lru ----------

#[test]
fn lru_new_is_empty() {
    let mut lru: Lru<u32, String> = Lru::new();
    assert!(!lru.contains(0));
    assert!(lru.get_mut(0).is_none());
}

#[test]
fn lru_insert_and_get() {
    let mut lru: Lru<u32, String> = Lru::new();
    assert!(lru.insert(1, "a".to_string()).is_none());
    assert!(lru.contains(1));
    assert_eq!(lru.get_mut(1).map(|s| s.as_str()), Some("a"));
}

#[test]
fn lru_insert_replaces_existing() {
    let mut lru: Lru<u32, String> = Lru::new();
    lru.insert(1, "old".to_string());
    let prev = lru.insert(1, "new".to_string());
    assert_eq!(prev, Some("old".to_string()));
    assert_eq!(lru.get_mut(1).map(|s| s.as_str()), Some("new"));
}

#[test]
fn lru_contains_returns_correct_state() {
    let mut lru: Lru<u32, &str> = Lru::new();
    lru.insert(1, "a");
    assert!(lru.contains(1));
    assert!(!lru.contains(2));
}

#[test]
fn lru_evict_if_over_removes_oldest() {
    let mut lru: Lru<u32, String> = Lru::new();
    lru.insert(1, "a".to_string());
    lru.insert(2, "b".to_string());
    lru.insert(3, "c".to_string());

    // 超出 max=2，淘汰最久未使用的 key=1
    let evicted = lru.evict_if_over(2);
    assert_eq!(evicted, Some("a".to_string()));
    assert!(!lru.contains(1));
    assert!(lru.contains(2));
    assert!(lru.contains(3));
}

#[test]
fn lru_evict_if_over_noop_when_under_limit() {
    let mut lru: Lru<u32, String> = Lru::new();
    lru.insert(1, "a".to_string());
    assert!(lru.evict_if_over(2).is_none());
    assert!(lru.contains(1));
}

#[test]
fn lru_evict_if_over_noop_when_at_limit() {
    let mut lru: Lru<u32, String> = Lru::new();
    lru.insert(1, "a".to_string());
    lru.insert(2, "b".to_string());
    assert!(lru.evict_if_over(2).is_none());
}

#[test]
fn lru_touch_on_get_updates_recency() {
    let mut lru: Lru<u32, String> = Lru::new();
    lru.insert(1, "a".to_string());
    lru.insert(2, "b".to_string());

    // 访问 key=1，使其成为最近使用
    lru.get_mut(1);

    // 现在淘汰应移除 key=2（最久未使用）
    let evicted = lru.evict_if_over(1);
    assert_eq!(evicted, Some("b".to_string()));
    assert!(lru.contains(1));
    assert!(!lru.contains(2));
}

#[test]
fn lru_clear_removes_all() {
    let mut lru: Lru<u32, String> = Lru::new();
    lru.insert(1, "a".to_string());
    lru.insert(2, "b".to_string());
    lru.clear();
    assert!(!lru.contains(1));
    assert!(!lru.contains(2));
}

#[test]
fn lru_multiple_evictions() {
    let mut lru: Lru<u32, String> = Lru::new();
    for i in 0..5 {
        lru.insert(i, format!("v{}", i));
    }
    // max=2，需要淘汰 3 条
    let mut evicted_count = 0;
    while lru.evict_if_over(2).is_some() {
        evicted_count += 1;
    }
    assert_eq!(evicted_count, 3);
    assert!(lru.contains(3));
    assert!(lru.contains(4));
    assert!(!lru.contains(0));
    assert!(!lru.contains(1));
    assert!(!lru.contains(2));
}

// ---------- Lru（迁移自内联测试）----------

#[test]
fn hit_moves_entry_to_back() {
    let mut lru = Lru::new();
    lru.insert(1u64, "a");
    lru.insert(2u64, "b");
    assert!(lru.get_mut(1).is_some());
    assert_eq!(lru.evict_if_over(2), None);

    lru.insert(3u64, "c");
    assert_eq!(lru.evict_if_over(2), Some("b"));
}

#[test]
fn evicts_oldest_when_over_capacity() {
    let mut lru = Lru::new();
    lru.insert(1u64, 10);
    lru.insert(2u64, 20);
    lru.insert(3u64, 30);

    assert_eq!(lru.evict_if_over(2), Some(10));
    assert_eq!(lru.evict_if_over(2), None);
}

#[test]
fn clear_removes_all_entries() {
    let mut lru = Lru::new();
    lru.insert(1u64, 10);
    lru.insert(2u64, 20);

    lru.clear();

    assert!(!lru.contains(1));
    assert!(!lru.contains(2));
}
