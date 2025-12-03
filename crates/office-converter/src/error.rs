use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("注册表错误: {0}")]
    Registry(#[from] std::io::Error),

    #[error("不支持的文件格式: {0}")]
    UnsupportedFormat(String),

    #[error("未安装办公软件")]
    NoOfficeInstalled,

    #[error("文档转换失败: {0}")]
    ConversionFailed(String),

    #[error("Windows COM 错误: {0}")]
    ComError(#[from] windows::core::Error),

    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("JSON 序列化错误: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
