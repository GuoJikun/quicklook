use std::path::Path;

use quicklook_video::{VideoEncoder, gpu_name_contains, is_compatible_video, target_bitrate};

// ---------- is_compatible_video ----------

#[test]
fn compatible_mp4_h264_returns_true() {
    assert!(is_compatible_video(Path::new("test.mp4"), "mp4", "h264"));
}

#[test]
fn incompatible_extension_returns_false() {
    assert!(!is_compatible_video(Path::new("test.mkv"), "mkv", "h264"));
    assert!(!is_compatible_video(Path::new("test.avi"), "avi", "h264"));
}

#[test]
fn incompatible_codec_returns_false() {
    assert!(!is_compatible_video(Path::new("test.mp4"), "mp4", "hevc"));
    assert!(!is_compatible_video(Path::new("test.mp4"), "mp4", "vp9"));
}

#[test]
fn empty_params_return_false() {
    assert!(!is_compatible_video(Path::new("test.mp4"), "", "h264"));
    assert!(!is_compatible_video(Path::new("test.mp4"), "mp4", ""));
    assert!(!is_compatible_video(Path::new("test.mp4"), "", ""));
}

// ---------- gpu_name_contains ----------

#[test]
fn gpu_name_contains_matches_keyword() {
    let names = vec!["NVIDIA GeForce RTX 3080".to_string()];
    assert!(gpu_name_contains(&names, "NVIDIA"));
    assert!(gpu_name_contains(&names, "GeForce"));
}

#[test]
fn gpu_name_contains_is_case_insensitive() {
    let names = vec!["NVIDIA GeForce RTX 3080".to_string()];
    assert!(gpu_name_contains(&names, "nvidia"));
    assert!(gpu_name_contains(&names, "geforce"));
    assert!(gpu_name_contains(&names, "NVIDIA"));
}

#[test]
fn gpu_name_contains_returns_false_when_no_match() {
    let names = vec!["Intel UHD Graphics".to_string()];
    assert!(!gpu_name_contains(&names, "NVIDIA"));
    assert!(!gpu_name_contains(&names, "AMD"));
}

#[test]
fn gpu_name_contains_empty_list_returns_false() {
    assert!(!gpu_name_contains(&[], "NVIDIA"));
}

#[test]
fn gpu_name_contains_multiple_names() {
    let names = vec![
        "Intel UHD Graphics".to_string(),
        "NVIDIA GeForce RTX 3080".to_string(),
    ];
    assert!(gpu_name_contains(&names, "Intel"));
    assert!(gpu_name_contains(&names, "NVIDIA"));
    assert!(!gpu_name_contains(&names, "AMD"));
}

// ---------- VideoEncoder ----------

#[test]
fn codec_name_maps_all_variants() {
    assert_eq!(VideoEncoder::Nvenc.codec_name(), "h264_nvenc");
    assert_eq!(VideoEncoder::Amf.codec_name(), "h264_amf");
    assert_eq!(VideoEncoder::Qsv.codec_name(), "h264_qsv");
    assert_eq!(VideoEncoder::MediaFoundation.codec_name(), "h264_mf");
}

#[test]
fn hardware_encoders_fall_back_to_media_foundation() {
    let target = Some(VideoEncoder::MediaFoundation);
    assert_eq!(VideoEncoder::Nvenc.fallback(), target);
    assert_eq!(VideoEncoder::Amf.fallback(), target);
    assert_eq!(VideoEncoder::Qsv.fallback(), target);
}

#[test]
fn media_foundation_is_last_resort() {
    assert_eq!(VideoEncoder::MediaFoundation.fallback(), None);
}

#[test]
fn encoder_equality() {
    assert_eq!(VideoEncoder::Nvenc, VideoEncoder::Nvenc);
    assert_ne!(VideoEncoder::Nvenc, VideoEncoder::Amf);
    assert_ne!(VideoEncoder::Qsv, VideoEncoder::MediaFoundation);
}

// ---------- target_bitrate ----------

#[test]
fn target_bitrate_scales_with_resolution() {
    assert_eq!(target_bitrate(3840, 2160), "16M");
    assert_eq!(target_bitrate(2560, 1440), "8M");
    assert_eq!(target_bitrate(1920, 1080), "4M");
    assert_eq!(target_bitrate(1280, 720), "2M");
    assert_eq!(target_bitrate(640, 360), "1M");
}

#[test]
fn target_bitrate_handles_degenerate_size() {
    assert_eq!(target_bitrate(0, 0), "1M");
    assert_eq!(target_bitrate(u32::MAX, u32::MAX), "16M");
}
