use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::os::windows::fs::MetadataExt;
use std::path::Path;
use std::sync::LazyLock;

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

/// 无后缀文件名 → Shiki 语言映射
static BUILD_NAME_TO_LANG: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        HashMap::from([
            // --- 文档类 ---
            ("README", "markdown"),
            ("CHANGELOG", "markdown"),
            ("TODO", "markdown"),
            ("LICENSE", "text"),
            ("COPYING", "text"),
            ("AUTHORS", "text"),
            ("CONTRIBUTORS", "text"),
            // --- 构建工具 ---
            ("Makefile", "makefile"),
            ("Dockerfile", "docker"),
            ("Jenkinsfile", "groovy"),
            ("Rakefile", "ruby"),
            ("Gemfile", "ruby"),
            ("Capfile", "ruby"),
            ("Podfile", "ruby"),
            ("Fastfile", "ruby"),
            ("Guardfile", "ruby"),
            ("Brewfile", "ruby"),
            ("Procfile", "yaml"),
            ("Vagrantfile", "ruby"),
            ("Caskfile", "ruby"),
            ("Appfile", "ruby"),
            ("Dangerfile", "ruby"),
            ("Deliverfile", "ruby"),
            ("Snapfile", "ruby"),
            // --- 配置/环境 ---
            (".npmrc", "ini"),
            (".yarnrc", "ini"),
            (".editorconfig", "ini"),
            (".gitconfig", "ini"),
            (".env", "ini"),
            // --- Shell 环境 ---
            (".bashrc", "bash"),
            (".zshrc", "bash"),
            (".profile", "bash"),
            (".bash_profile", "bash"),
            // --- 其他特殊 ---
            ("PKGBUILD", "bash"),     // Arch Linux
            ("Justfile", "makefile"), // just 任务文件
            ("Snakefile", "python"),  // Snakemake
            ("BUILD", "python"),      // Bazel
            ("WORKSPACE", "python"),  // Bazel
        ])
    });

/// 仅在文件没有后缀时调用
pub fn detect_language_no_ext(file_name: &str) -> String {
    log::debug!(
        "Detecting language for file without extension: {}",
        file_name
    );
    let map = &*BUILD_NAME_TO_LANG;
    map.get(file_name).copied().unwrap_or("txt").to_string()
}

pub fn get_file_info(
    path: &str,
    custom_code_exts: &[String],
    custom_video_exts: &[String],
) -> Option<File> {
    let file_path = Path::new(path);
    let path_str = path.to_string();

    // 如果不是文件则返回 None
    if file_path.is_file() == false {
        return None;
    }
    // 获取文件名称
    let name = match file_path.file_name() {
        Some(tmp) => tmp.to_string_lossy().into_owned(),
        None => String::from(""),
    };
    // 获取文件扩展名，如果没有扩展名，默认使用 "txt"
    let extension = match file_path.extension() {
        Some(ext) => ext.to_string_lossy().to_lowercase(),
        None => detect_language_no_ext(&name),
    };
    log::debug!("File extension: {}", extension);

    let metadata = file_path.metadata().unwrap();

    // 先从内置映射表中查找文件类型
    let file_type_opt = FILE_TYPE_MAPPING
        .get(extension.as_str())
        .map(|s| s.to_string());

    // 如果内置映射表中没有匹配，检查用户自定义扩展名
    let file_type_opt = file_type_opt.or_else(|| {
        if custom_code_exts.iter().any(|e| e.eq_ignore_ascii_case(&extension)) {
            Some("Code".to_string())
        } else if custom_video_exts.iter().any(|e| e.eq_ignore_ascii_case(&extension)) {
            Some("Video".to_string())
        } else {
            None
        }
    });

    match file_type_opt {
        Some(ref file_type) => Some(File::new(
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
static FILE_TYPE_MAPPING: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
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
            ("psd", "Image"),
            ("tiff", "Image"),
            ("tif", "Image"),
            ("tga", "Image"),
            ("pbm", "Image"),
            ("pgm", "Image"),
            ("ppm", "Image"),
            ("qoi", "Image"),
            ("exr", "Image"),
            ("heic", "Image"),
            ("heif", "Image"),
            ("jxl", "Image"),
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
            ("zst", "Archive"),  // zstandard 压缩文件
            ("tzst", "Archive"), // tar.zst 的简写
            ("cpio", "Archive"), // CPIO 归档
            ("ar", "Archive"),   // Unix ar 归档
            ("deb", "Archive"),  // Debian 软件包（外层 ar）
            ("a", "Archive"),    // 静态库
            // ZIP 本质但带特殊扩展名
            ("jar", "Archive"),  // Java 归档
            ("war", "Archive"),  // Java Web 归档
            ("ear", "Archive"),  // Java Enterprise 归档
            ("apk", "Archive"),  // Android 应用包
            ("aar", "Archive"),  // Android 归档
            ("whl", "Archive"),  // Python wheel
            ("vsix", "Archive"), // VS Code 扩展
            ("nupkg", "Archive"),// NuGet 包
            ("crx", "Archive"),  // Chrome 扩展
            ("xpi", "Archive"),  // Firefox 扩展
            ("egg", "Archive"),  // Python egg
            ("kra", "Archive"),  // Krita 文档
            ("xps", "Archive"),  // XML Paper Specification
            ("oxps", "Archive"), // OpenXPS
            // 书籍文件
            ("pdf", "Book"),
            // 代码文件
            ("txt", "Code"),
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
            ("bash", "Code"),
            // 应用程序文件
            // ("exe", "App"),
            // ("dmg", "App"),
            // ("deb", "App"),
            // ("rpm", "App"),
            // ("apk", "App"),
            // ("appimage", "App"),
        ])
    });
