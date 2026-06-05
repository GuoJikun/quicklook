use quicklook_archive::{extractors, Extract};
use tauri::command;

use crate::error::QuickLookError;

#[command]
pub fn archive(path: &str, mode: &str) -> Result<Vec<Extract>, QuickLookError> {
    log::info!("开始处理压缩文件: {}, 扩展名: {}", path, mode);
    let result = match mode {
        "zip" => extractors::zip::zip_extract(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "tar" => extractors::tar::list_tar_entries(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "gz" | "tgz" => extractors::tar::list_tar_gz_entries(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "bz2" | "tbz2" => extractors::tar::list_tar_bz2_entries(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "xz" | "txz" => extractors::tar::list_tar_xz_entries(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "zst" | "tzst" => extractors::zst::list_tar_zst_entries(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "7z" => extractors::sevenz::list_7z_entries(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "rar" => extractors::rar::list_rar_entries(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "cpio" => extractors::cpio::list_cpio_entries(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "ar" | "deb" | "a" => extractors::ar::list_ar_entries(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        "jar" | "war" | "ear" | "apk" | "aar" | "whl" | "vsix" | "nupkg" | "crx" | "xpi"
        | "egg" | "kra" | "xps" | "oxps" => extractors::zip::zip_extract(path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string())),
        _ => return Err(QuickLookError::UnsupportedArchiveFormat(mode.to_string())),
    };

    match &result {
        Ok(entries) => {
            log::info!("成功处理压缩文件，共{}个条目", entries.len());
        },
        Err(e) => {
            log::error!("压缩文件处理失败: {}", e);
        },
    }

    result
}
