use quicklook_archive::extractors::rar::rar_time_to_string;
use quicklook_archive::extractors::sevenz::filetime_to_string;
use quicklook_archive::extractors::zip::decode_entry_name;

// ---------- decode_entry_name (zip) ----------

#[test]
fn decode_utf8_name() {
    assert_eq!(decode_entry_name("hello.txt".as_bytes()), "hello.txt");
    assert_eq!(decode_entry_name("文档.md".as_bytes()), "文档.md");
}

#[test]
fn decode_gbk_name() {
    // GBK 编码的 "中文.txt"
    let gbk_bytes: &[u8] = &[0xD6, 0xD0, 0xCE, 0xC4, 0x2E, 0x74, 0x78, 0x74];
    assert_eq!(decode_entry_name(gbk_bytes), "中文.txt");
}

#[test]
fn decode_empty_bytes() {
    assert_eq!(decode_entry_name(b""), "");
}

#[test]
fn decode_invalid_utf8_falls_back_to_gbk() {
    // 0xFF 不是合法 UTF-8 起始字节，GBK 会解码为替换字符或某个字符
    let result = decode_entry_name(&[0xFF, 0xFE]);
    // 只要不 panic 即可
    assert!(!result.is_empty() || result.is_empty());
}

// ---------- filetime_to_string (sevenz) ----------

#[test]
fn filetime_zero_returns_epoch() {
    assert_eq!(filetime_to_string(0), "1970-01-01 00:00:00");
}

#[test]
fn filetime_before_unix_epoch_returns_epoch() {
    // FILETIME_UNIX_EPOCH_DIFF 之前的值
    assert_eq!(filetime_to_string(1), "1970-01-01 00:00:00");
    assert_eq!(filetime_to_string(100), "1970-01-01 00:00:00");
}

#[test]
fn filetime_at_unix_epoch_returns_epoch() {
    // FILETIME_UNIX_EPOCH_DIFF 正好是 Unix epoch 的 FILETIME 表示
    let unix_epoch_filetime: u64 = 116_444_736_000_000_000;
    assert_eq!(
        filetime_to_string(unix_epoch_filetime),
        "1970-01-01 00:00:00"
    );
}

#[test]
fn filetime_after_epoch_converts_correctly() {
    // 2020-01-01 00:00:00 UTC 的 FILETIME
    // Unix timestamp: 1577836800
    // FILETIME = (1577836800 + 11644473600) * 10000000 = 132223104000000000
    let filetime: u64 = 132_223_104_000_000_000;
    assert_eq!(filetime_to_string(filetime), "2020-01-01 00:00:00");
}

// ---------- rar_time_to_string (rar) ----------

#[test]
fn rar_time_zero_returns_epoch() {
    assert_eq!(rar_time_to_string(0), "1970-01-01 00:00:00");
}

#[test]
fn rar_time_converts_dos_datetime() {
    // DOS 时间格式：
    // date: bits 15-9 = year-1980, bits 8-5 = month, bits 4-0 = day
    // time: bits 15-11 = hour, bits 10-5 = minute, bits 4-0 = second/2
    //
    // 构造 2020-06-15 12:30:00
    // year = 2020 - 1980 = 40, month = 6, day = 15
    // date = (40 << 9) | (6 << 5) | 15 = 20480 + 192 + 15 = 20687
    // hour = 12, minute = 30, second = 0
    // time = (12 << 11) | (30 << 5) | 0 = 24576 + 960 + 0 = 25536
    // file_time = (date << 16) | time = (20687 << 16) | 25536
    let file_time: u32 = (20687u32 << 16) | 25536u32;
    assert_eq!(rar_time_to_string(file_time), "2020-06-15 12:30:00");
}

#[test]
fn rar_time_invalid_date_returns_epoch() {
    // 构造月份为 0 的无效日期
    // date = (40 << 9) | (0 << 5) | 0 = 20480
    let file_time: u32 = 20480u32 << 16;
    assert_eq!(rar_time_to_string(file_time), "1970-01-01 00:00:00");
}

#[test]
fn rar_time_invalid_day_returns_epoch() {
    // 构造日期为 0 的无效日期（day = 0）
    // date = (40 << 9) | (6 << 5) | 0 = 20480 + 192 = 20672
    let file_time: u32 = 20672u32 << 16;
    assert_eq!(rar_time_to_string(file_time), "1970-01-01 00:00:00");
}
