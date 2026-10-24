use quicklook_error::QuickLookError;

#[derive(Debug)]
struct RootCause;

impl std::fmt::Display for RootCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "root cause")
    }
}

impl std::error::Error for RootCause {}

#[test]
fn with_source_preserves_error_chain() {
    let err = QuickLookError::with_source("压缩文件解析失败", RootCause);

    assert_eq!(err.to_string(), "压缩文件解析失败: root cause");
    assert_eq!(
        std::error::Error::source(&err).unwrap().to_string(),
        "root cause"
    );
}
