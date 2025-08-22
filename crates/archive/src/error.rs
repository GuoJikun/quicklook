use std::fmt;

#[derive(Debug)]
pub enum ArchiveError {
    IoError(std::io::Error),
    ZipError(zip::result::ZipError),
    SevenZError(sevenz_rust::Error),
    UnsupportedFormat(String),
    InvalidPath(String),
    Other(String),
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveError::IoError(err) => write!(f, "IO error: {}", err),
            ArchiveError::ZipError(err) => write!(f, "ZIP error: {}", err),
            ArchiveError::SevenZError(err) => write!(f, "7Z error: {}", err),
            ArchiveError::UnsupportedFormat(format) => write!(f, "Unsupported format: {}", format),
            ArchiveError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            ArchiveError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ArchiveError {}

impl From<std::io::Error> for ArchiveError {
    fn from(err: std::io::Error) -> Self {
        ArchiveError::IoError(err)
    }
}

impl From<zip::result::ZipError> for ArchiveError {
    fn from(err: zip::result::ZipError) -> Self {
        ArchiveError::ZipError(err)
    }
}

impl From<sevenz_rust::Error> for ArchiveError {
    fn from(err: sevenz_rust::Error) -> Self {
        ArchiveError::SevenZError(err)
    }
}
