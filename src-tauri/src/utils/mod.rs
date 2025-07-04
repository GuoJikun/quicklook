use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::os::windows::fs::MetadataExt;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct File {
    file_type: String,
    path: String,
    extension: String,
    size: u64,
    last_modified: u64,
    name: String,
}

impl File {
    // 构造 File 实例
    fn new(
        file_type: &str,
        path: String,
        extension: String,
        size: u64,
        last_modified: u64,
        name: String,
    ) -> File {
        File {
            file_type: file_type.to_string(),
            path,
            extension,
            size,
            last_modified,
            name,
        }
    }

    // 获取文件类型
    pub fn get_file_type(&self) -> String {
        self.file_type.clone()
    }

    // 获取文件路径
    pub fn get_path(&self) -> String {
        self.path.clone()
    }

    // 获取文件扩展名
    pub fn get_extension(&self) -> String {
        self.extension.clone()
    }

    // 获取文件大小
    pub fn get_size(&self) -> u64 {
        self.size
    }

    // 获取文件最后修改时间
    pub fn get_last_modified(&self) -> u64 {
        self.last_modified
    }

    // 获取文件名
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}

pub fn get_file_info(path: &str) -> Option<File> {
    let file_path = Path::new(path);
    let path_str = path.to_string();

    // 如果不是文件则返回 None
    if file_path.is_file() == false {
        return None;
    }
    // 获取文件扩展名，如果没有扩展名，默认使用 "txt"
    let extension = file_path
        .extension()
        .map_or("txt".to_string(), |ext| ext.to_string_lossy().into_owned());

    let metadata = file_path.metadata().unwrap();
    let name = match file_path.file_name() {
        Some(tmp) => tmp.to_string_lossy().into_owned(),
        None => String::from(""),
    };
    // 根据扩展名从映射表中获取文件类型
    match file_type_mapping().get(extension.as_str()) {
        Some(file_type) => Some(File::new(
            file_type,
            path_str,
            extension,
            metadata.file_size(),
            metadata.last_write_time(),
            name,
        )),
        None => None, // 如果没有匹配的文件类型，返回 None
    }
}

// 返回一个文件扩展名到文件类型的映射
fn file_type_mapping() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        // md
        ("markdown", "Markdown"),
        ("md", "Markdown"),
        // DOC 文件
        // ("doc", "Doc"),
        ("docx", "Doc"),
        ("xls", "Doc"),
        ("xlsx", "Doc"),
        ("xlsm", "Doc"),
        ("xlsb", "Doc"),
        ("xla", "Doc"),
        ("xlam", "Doc"),
        ("ods", "Doc"),
        ("csv", "Doc"),
        // ("ppt", "Doc"),
        // ("pptx", "Doc"),
        // 字体文件
        ("ttf", "Font"),
        ("otf", "Font"),
        ("woff2", "Font"),
        ("woff", "Font"),
        // 图片文件
        ("jpg", "Image"),
        ("jpeg", "Image"),
        ("png", "Image"),
        ("gif", "Image"),
        ("webp", "Image"),
        ("bmp", "Image"),
        ("ico", "Image"),
        ("svg", "Image"),
        ("apng", "Image"),
        // 视频文件
        ("mp4", "Video"),
        ("webm", "Video"),
        ("mkv", "Video"),
        ("avi", "Video"),
        ("mov", "Video"),
        ("wmv", "Video"),
        ("mpg", "Video"),
        ("mpeg", "Video"),
        ("m4v", "Video"),
        ("3gp", "Video"),
        ("3g2", "Video"),
        // 音频文件
        ("mp3", "Audio"),
        ("3gp", "Audio"), // 注意：3gp 可能同时属于视频和音频
        ("ogg", "Audio"),
        ("m4a", "Audio"),
        // 压缩文件
        ("7z", "Archive"),
        ("zip", "Archive"),
        ("tar", "Archive"),
        ("gz", "Archive"),
        ("tgz", "Archive"),  // tar.gz 的简写
        ("bz2", "Archive"),  // bzip2 压缩文件
        ("tbz2", "Archive"), // tar.bz2 的简写
        ("xz", "Archive"),   // xz 压缩文件
        ("txz", "Archive"),  // tar.xz 的简写
        // 书籍文件
        ("pdf", "Book"),
        // 文本文件
        ("txt", "Text"),
        // 代码文件
        ("cpp", "Code"),
        ("js", "Code"),
        ("mjs", "Code"),
        ("cjs", "Code"),
        ("ts", "Code"),
        ("mts", "Code"),
        ("tsx", "Code"),
        ("rs", "Code"),
        ("py", "Code"),
        ("java", "Code"),
        ("html", "Code"),
        ("css", "Code"),
        ("scss", "Code"),
        ("sass", "Code"),
        ("less", "Code"),
        ("styl", "Code"),
        ("c", "Code"),
        ("cs", "Code"),
        ("go", "Code"),
        ("vue", "Code"),
        ("svelte", "Code"),
        ("astro", "Code"),
        ("jsx", "Code"),
        ("json", "Code"),
        ("yml", "Code"),
        ("yaml", "Code"),
        ("toml", "Code"),
        ("bat", "Code"),
        ("ps1", "Code"),
        ("ini", "Code"),
        ("swift", "Code"),
        ("kt", "Code"),
        ("php", "Code"),
        ("h", "Code"),
        ("xml", "Code"),
        ("sql", "Code"),
        ("pug", "Code"),
        ("lua", "Code"),
        ("r", "Code"),
        ("d", "Code"),
        ("vb", "Code"),
        ("pas", "Code"),
        ("scala", "Code"),
        ("m", "Code"),
        ("log", "Code"),
        // 应用程序文件
        // ("exe", "App"),
        // ("dmg", "App"),
        // ("deb", "App"),
        // ("rpm", "App"),
        // ("apk", "App"),
        // ("appimage", "App"),
    ])
}
