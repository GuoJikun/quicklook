use std::{env, path::{PathBuf, Path}};

fn main() {
    // 设置环境变量
    env::set_var("RUST_BACKTRACE", "full");
    env::set_var("VCPKGRS_DYNAMIC", "1");  // 明确使用动态链接
    env::set_var("CARGO_PROFILE_DEV_BUILD_OVERRIDE_DEBUG", "true");

    // 设定 vcpkg 的安装目录
    let vcpkg_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent();
    if vcpkg_root.is_none() {
        panic!("Failed to get vcpkg root directory");
    }
    let vcpkg_dir = vcpkg_root.unwrap().join("vcpkg").to_string_lossy().to_string();
    println!("cargo:warning=Using VCPKG_ROOT: {}", vcpkg_dir);

    // 配置 vcpkg 环境
    println!("cargo:rerun-if-env-changed=VCPKG_ROOT");
    println!("cargo:rerun-if-changed=build.rs");

    // 查找 ffmpeg 和 libarchive
    let mut binding = vcpkg::Config::new();
    let config = binding
        .emit_includes(true)
        .cargo_metadata(true);
    #[cfg(any(target_os = "windows", target_arch = "x86_64"))]
    let config = config.target_triplet("x64-windows");
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    let config = config.target_triplet("arm64-windows");
    
    // config.find_package("ffmpeg").expect("Failed to find ffmpeg");
    config.find_package("libarchive").expect("Failed to find libarchive");

    // 生成绑定
    #[cfg(any(target_os = "windows", target_arch = "x86_64"))]
    let vcpkg_include = format!("{}/vcpkg_installed/x64-windows/include", vcpkg_dir);
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    let vcpkg_include = format!("{}/vcpkg_installed/arm64-windows/include", vcpkg_dir);

    // 检查头文件是否存在
    let archive_h = format!("{}/archive.h", vcpkg_include);
    let archive_entry_h = format!("{}/archive_entry.h", vcpkg_include);
    
    if !std::path::Path::new(&archive_h).exists() {
        panic!("archive.h not found at {}", archive_h);
    }
    if !std::path::Path::new(&archive_entry_h).exists() {
        panic!("archive_entry.h not found at {}", archive_entry_h);
    }

    let bindings = bindgen::Builder::default()
        .header(archive_h)
        .header(archive_entry_h)
        .clang_arg(format!("-I {}", vcpkg_include))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate libarchive bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("libarchive.rs"))
        .expect("Failed to write libarchive bindings");
    
    tauri_build::build()
}
