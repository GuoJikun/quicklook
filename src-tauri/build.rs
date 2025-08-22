fn main() {
    // Configure vcpkg for FFmpeg dependencies
    #[cfg(target_os = "windows")]
    {
        // Try to use vcpkg to find FFmpeg libraries
        if let Ok(_) = std::env::var("VCPKG_ROOT") {
            vcpkg::Config::new()
                .cargo_metadata(true)
                .probe("ffmpeg")
                .ok();
        }
    }
    
    tauri_build::build()
}
