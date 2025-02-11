use std::fs::File;
use std::ffi::{CStr, CString};
use serde::Serialize;
use zip::{DateTime, ZipArchive};

#[path ="../bindings.rs"]
mod binds;
use binds::libarchive;

#[allow(unused)]
#[derive(Debug, Clone, Serialize)]
pub struct Extract {
    name: String,
    size: u64,
    last_modified: String,
    dir: bool,
}

#[allow(unused)]
impl Extract {
    pub fn zip(zip_path: &str) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
        // 打开压缩文件
        let file = File::open(zip_path)?;
        let mut archive: ZipArchive<File> = ZipArchive::new(file)?;
        let mut target: Vec<Extract> = vec![];
        // 遍历压缩包中的每一个文件
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let dir = file.is_dir();
            let file_name = file.name().to_string();
            let size = file.size();
            let last_modified = file
                .last_modified()
                .unwrap_or(DateTime::default())
                .to_string();

            target.push(Extract {
                name: file_name,
                size,
                last_modified,
                dir,
            });
        }

        let archives = Extract::new(zip_path);
        println!("{:?}", archives);

        Ok(target)
    }

    pub fn new(path: &str) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
        let mut entries: Vec<Extract> = vec![];
        
        unsafe {
            // 创建新的归档对象
            let archive = match libarchive::archive_read_new() {
                ptr if ptr.is_null() => return Err("Failed to create archive".into()),
                ptr => ptr
            };
            // 初始化过滤器和格式支持
            if libarchive::archive_read_support_filter_all(archive) != libarchive::ARCHIVE_OK as i32 {
                libarchive::archive_read_close(archive);
                libarchive::archive_read_free(archive);
                return Err("Failed to enable filters".into());
            }

            if libarchive::archive_read_support_format_all(archive) != libarchive::ARCHIVE_OK as i32 {
                libarchive::archive_read_close(archive);
                libarchive::archive_read_free(archive);
                return Err("Failed to enable formats".into());
            }
            // 打开文件
            let c_path = match CString::new(path) {
                Ok(p) => p,
                Err(_) => {
                    libarchive::archive_read_close(archive);
                    libarchive::archive_read_free(archive);
                    return Err("Invalid path".into());
                }
            };
            
            if libarchive::archive_read_open_filename(archive, c_path.as_ptr(), 10240) != libarchive::ARCHIVE_OK as i32 {
                libarchive::archive_read_close(archive);
                libarchive::archive_read_free(archive);
                return Err("Failed to open archive".into());
            }

            loop {
                let mut entry = std::ptr::null_mut();
                match libarchive::archive_read_next_header(archive, &mut entry) {
                    r if r == libarchive::ARCHIVE_EOF as i32 => break,
                    r if r != libarchive::ARCHIVE_OK as i32 => {
                        libarchive::archive_read_close(archive);
                        libarchive::archive_read_free(archive);
                        return Err("Failed to read entry".into());
                    },
                    _ => {}
                }

                let name = libarchive::archive_entry_pathname(entry);
                let name_str =  CStr::from_ptr(name).to_string_lossy().into_owned();
                let size = libarchive::archive_entry_size(entry);
                let last_modified = libarchive::archive_entry_mtime(entry);
                let dir_tmp = libarchive::archive_entry_filetype(entry);
                let dir = dir_tmp as u32 == libarchive::S_IFDIR;
                entries.push(Extract {
                    name: name_str,
                    size: size as u64,
                    last_modified: last_modified.to_string(),
                    dir,
                });
            }
            libarchive::archive_read_close(archive);
            libarchive::archive_read_free(archive);
        }

        Ok(entries)
    }
}
