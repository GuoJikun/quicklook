// chrono 仅在多 run 目录方案时需要，恢复固定目录后可移除
use ez_ffmpeg::core::codec::{get_decoders, get_encoders};
use ez_ffmpeg::core::context::output::VSyncMethod;
use ez_ffmpeg::core::stream_info::{find_video_stream_info, StreamInfo};
use ez_ffmpeg::{AVRational, FfmpegContext, FfmpegScheduler, Output};
use std::{env, fs, path::Path, path::PathBuf, thread};
use tiny_http::{Response, Server};

pub fn detect_supported_hw_decode() -> Option<String> {
    let mut names: Vec<String> = get_decoders()
        .into_iter()
        .map(|c| c.codec_name.to_ascii_lowercase())
        .collect();
    names.extend(
        get_encoders()
            .into_iter()
            .map(|c| c.codec_name.to_ascii_lowercase()),
    );

    let contains_any = |keys: &[&str]| -> bool {
        names
            .iter()
            .any(|name| keys.iter().any(|key| name.contains(key)))
    };

    if contains_any(&["cuvid", "nvdec", "nvenc", "nvcodec"]) {
        return Some("nvcodec".to_string());
    }
    if contains_any(&["qsv"]) {
        return Some("qsv".to_string());
    }
    if contains_any(&["amf"]) {
        return Some("amf".to_string());
    }

    Some("openh264".to_string())
}

fn generate_hls(input_file: &str, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&output_dir)?;
    let m3u8_path = output_dir.join("playlist.m3u8");

    println!("🔄 强制转码模式: 确保最大Web兼容性");

    // 探测帧率
    let mut target_fps: i32 = 25;
    if let Ok(Some(StreamInfo::Video { fps, .. })) = find_video_stream_info(input_file) {
        let rounded = fps.round() as i32;
        if (5..=120).contains(&rounded) {
            target_fps = rounded;
        }
        println!("🎯 探测到输入帧率: {:.3} -> 使用 {}fps", fps, target_fps);
    } else {
        println!("⚠️ 未能探测到视频帧率，使用默认 {}fps", target_fps);
    }

    // 源帧率浮点（之前只保留整数）；这里重新获取原始浮点便于判断是否稳定
    let source_fps = target_fps as f64; // 如果后续能拿到更精确的浮点 fps，可替换
    let is_near_integer = (source_fps - source_fps.round()).abs() < 0.01; // 近似整数帧率
    let low_fps = source_fps < 15.0; // 低帧率判定
                                     // 策略：低帧率或非整数帧率 => VFR；其余 => CFR
    let use_vfr = low_fps || !is_near_integer;
    let hls_time_val = if low_fps { "2" } else { "4" }; // 低帧率缩短分片提升时间精度
    if use_vfr {
        println!(
            "🌀 使用 VFR 模式 (low_fps={}, near_integer={})",
            low_fps, is_near_integer
        );
    } else {
        println!("📏 使用 CFR 模式: {:.3} fps", source_fps);
    }

    // 仅在使用 CFR 时设置精确帧率 (整数分数 num/den=round fps /1)
    let fps_rational = AVRational { num: source_fps.round() as i32, den: 1 };

    let video_codecs = vec!["h264", "mpeg4", "libvpx", "libvpx-vp9"]; // 优先级
    let audio_codecs = vec!["mp3", "aac", "libmp3lame"]; // 音频优先级

    let mut last_error: Option<Box<dyn std::error::Error>> = None;

    for video_codec in &video_codecs {
        for audio_codec in &audio_codecs {
            let mut output = Output::new(m3u8_path.to_str().unwrap())
                .set_format("hls")
                .add_stream_map("0:v?")
                .add_stream_map("0:a?")
                .set_video_codec(*video_codec)
                .set_audio_codec(*audio_codec)
                .set_format_opt("hls_time", hls_time_val)
                .set_format_opt("hls_list_size", "0")
                // 去掉 independent_segments 让编码器自然关键帧分布，观察是否减少 duration=0 包
                .set_format_opt("hls_flags", "append_list")
                .set_format_opt("hls_playlist_type", "vod")
                .set_format_opt("hls_segment_type", "mpegts")
                .set_format_opt("start_number", "0")
                .set_format_opt("hls_allow_cache", "1")
                .set_format_opt(
                    "hls_segment_filename",
                    output_dir.join("segment_%03d.ts").to_str().unwrap(),
                );

            // 允许通过环境变量打开时间戳复制与零基对齐，进一步保障时间精度（默认关闭保持最小改动）
            if let Ok(v) = std::env::var("QUICKLOOK_HLS_COPYTS") {
                if matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON") {
                    output = output
                        .set_format_opt("copyts", "1")
                        .set_format_opt("start_at_zero", "1");
                }
            }

            if use_vfr {
                output = output.set_vsync_method(VSyncMethod::VsyncVfr);
            } else {
                output = output
                    .set_framerate(fps_rational)
                    .set_vsync_method(VSyncMethod::VsyncCfr);
            }

            if video_codec.contains("libvpx") {
                output = output
                    .set_video_codec_opt("deadline", "realtime")
                    .set_video_codec_opt("cpu-used", "8")
                    .set_video_codec_opt("crf", "28");
            } else if *video_codec == "mpeg4" {
                output = output.set_video_codec_opt("qscale", "10");
            } else {
                // h264
                output = output
                    .set_video_codec_opt("b:v", "2000k")
                    .set_video_codec_opt("crf", "28")
                    .set_video_codec_opt("bf", "0")
                    .set_video_codec_opt("preset", "ultrafast")
                    .set_video_codec_opt("tune", "zerolatency")
                    .set_video_codec_opt("x264opts", "no-scenecut");
                // 移除 force_key_frames，避免人为插入导致相邻重复 PTS -> duration=0
            }

            output = output
                .set_audio_sample_rate(44100)
                .set_audio_codec_opt("b", "128k");

            if *audio_codec == "aac" {
                output = output
                    .set_audio_codec_opt("strict", "experimental")
                    .set_audio_codec_opt("ac", "2")
                    .set_audio_codec_opt("profile", "aac_low")
                    .set_audio_codec_opt("ar", "44100");
            } else if *audio_codec == "mp3" || audio_codec.contains("mp3") {
                output = output.set_audio_channels(2).set_audio_codec_opt("q", "4");
            } else {
                output = output.set_audio_channels(2);
            }

            let context_result = FfmpegContext::builder()
                .input(input_file)
                .output(output)
                .build();

            match context_result {
                Ok(context) => {
                    println!("✅ 使用编码器: 视频={}, 音频={}", video_codec, audio_codec);
                    match FfmpegScheduler::new(context).start() {
                        Ok(scheduler) => {
                            thread::spawn(move || match scheduler.wait() {
                                Ok(_) => println!("✅ HLS 转码完成"),
                                Err(e) => eprintln!("❌ HLS 转码错误: {:?}", e),
                            });
                            return Ok(());
                        },
                        Err(e) => {
                            last_error = Some(Box::new(e));
                            println!(
                                "⚠️ 编码器 {}+{} 启动失败，尝试下一个",
                                video_codec, audio_codec
                            );
                            continue;
                        },
                    }
                },
                Err(e) => {
                    last_error = Some(Box::new(e));
                    println!(
                        "⚠️ 编码器 {}+{} 构建失败，尝试下一个",
                        video_codec, audio_codec
                    );
                    continue;
                },
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "所有编码器都不可用",
        ))
    }))
}

fn start_http_server(hls_dir: PathBuf) {
    // 如端口已被占用(已启动)则跳过
    thread::spawn(move || {
        let server = match Server::http("127.0.0.1:17878") {
            Ok(s) => {
                println!("✅ HLS server at http://127.0.0.1:17878");
                s
            },
            Err(e) => {
                eprintln!("ℹ️ HLS server already running or cannot bind: {}", e);
                return; // 不重复启动
            },
        };

        for request in server.incoming_requests() {
            let path = request.url().trim_start_matches('/');
            let full_path = hls_dir.join(path);

            if full_path.exists() {
                match std::fs::read(&full_path) {
                    Ok(data) => {
                        let content_type = if path.ends_with(".m3u8") {
                            "application/vnd.apple.mpegurl"
                        } else if path.ends_with(".ts") {
                            "video/MP2T"
                        } else {
                            "application/octet-stream"
                        };

                        let response = Response::from_data(data)
                            .with_header(
                                tiny_http::Header::from_bytes(b"Content-Type", content_type)
                                    .unwrap(),
                            )
                            .with_header(
                                tiny_http::Header::from_bytes(b"Access-Control-Allow-Origin", b"*")
                                    .unwrap(),
                            )
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    b"Access-Control-Allow-Methods",
                                    b"GET, OPTIONS",
                                )
                                .unwrap(),
                            )
                            .with_header(
                                tiny_http::Header::from_bytes(b"Cache-Control", b"no-cache")
                                    .unwrap(),
                            );

                        if let Err(e) = request.respond(response) {
                            eprintln!("Failed to send response: {}", e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to read file {}: {}", full_path.display(), e);
                        let response =
                            Response::from_string("Internal Server Error").with_status_code(500);
                        let _ = request.respond(response);
                    },
                }
            } else {
                let response = Response::from_string("Not Found")
                    .with_status_code(404)
                    .with_header(
                        tiny_http::Header::from_bytes(b"Access-Control-Allow-Origin", b"*")
                            .unwrap(),
                    );
                let _ = request.respond(response);
            }
        }
    });
}

pub fn start_hls_process(input: String) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let output_dir = temp_dir.join("quicklook-hls-output");

    if output_dir.exists() {
        let _ = std::fs::remove_dir_all(&output_dir);
    }
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    // 启动（或复用）HTTP 服务
    start_http_server(output_dir.clone());

    println!(
        "[HLS] Single directory session input={} dir={}",
        input,
        output_dir.display()
    );

    let input_clone = input.clone();
    let dir_clone = output_dir.clone();
    std::thread::spawn(move || {
        if let Err(e) = generate_hls(&input_clone, &dir_clone) {
            eprintln!("❌ ffmpeg error: {:?}", e);
        } else {
            println!("✅ HLS 转码启动成功，目录: {}", dir_clone.display());
        }
    });

    Ok(format!(
        "HLS 服务已启动，播放地址：http://127.0.0.1:17878/playlist.m3u8\n文件缓存位置：{}",
        output_dir.display()
    ))
}
