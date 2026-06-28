use std::string::FromUtf8Error;

/// 统一错误类型，覆盖 quicklook 所有模块的错误场景。
#[derive(Debug, thiserror::Error)]
pub enum QuickLookError {
    // ── IO / 文件 ──────────────────────────────────
    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("IO 错误: {0}")]
    Io(String),

    // ── 压缩文件 ──────────────────────────────────
    #[error("不支持的压缩格式: {0}")]
    UnsupportedArchiveFormat(String),

    #[error("压缩文件解析失败: {0}")]
    ArchiveParse(String),

    // ── 文档 ──────────────────────────────────────
    #[error("文档解析失败: {0}")]
    DocumentParse(String),

    #[error("不支持的文档格式: {0}")]
    UnsupportedDocumentFormat(String),

    // ── 3D 模型 ────────────────────────────────────
    #[error("3D 模型解析失败: {0}")]
    ModelParse(String),

    #[error("不支持的 3D 模型格式: {0}")]
    UnsupportedModelFormat(String),

    // ── 图片 ──────────────────────────────────────
    #[error("图片处理失败: {0}")]
    ImageProcessing(String),

    // ── 音频 ──────────────────────────────────────
    #[error("音频元数据读取失败: {0}")]
    AudioMetadata(String),

    #[error("LRC 歌词解析失败: {0}")]
    LrcParse(String),

    // ── 视频 / ffmpeg ──────────────────────────────
    #[error("ffmpeg 未安装或不在 PATH 中")]
    FfmpegNotFound,

    #[error("视频转换失败: {0}")]
    VideoConversion(String),

    #[error("视频转换已取消")]
    VideoConversionCancelled,

    // ── 配置 ──────────────────────────────────────
    #[error("配置读取失败: {0}")]
    ConfigRead(String),

    #[error("配置字段缺失或类型错误: {0}")]
    ConfigField(String),

    // ── Windows 系统 ──────────────────────────────
    #[error("Windows API 错误: {0}")]
    WindowsApi(String),

    // ── PDF ──────────────────────────────────────
    #[error("PDF 渲染失败: {0}")]
    PdfRendering(String),

    #[error("PDF 大纲解析失败: {0}")]
    PdfOutline(String),

    // ── 通用 ──────────────────────────────────────
    #[error("UTF-8 解码错误: {0}")]
    Utf8(String),

    #[error("{0}")]
    Other(String),
}

impl serde::Serialize for QuickLookError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// 便捷 Result 别名。
pub type Result<T> = std::result::Result<T, QuickLookError>;

// ── From 实现，方便 ? 操作符自动转换 ──────────────

impl From<String> for QuickLookError {
    fn from(s: String) -> Self {
        QuickLookError::Other(s)
    }
}

impl From<&str> for QuickLookError {
    fn from(s: &str) -> Self {
        QuickLookError::Other(s.to_string())
    }
}

impl From<std::io::Error> for QuickLookError {
    fn from(e: std::io::Error) -> Self {
        QuickLookError::Io(e.to_string())
    }
}

impl From<FromUtf8Error> for QuickLookError {
    fn from(e: FromUtf8Error) -> Self {
        QuickLookError::Utf8(e.to_string())
    }
}
