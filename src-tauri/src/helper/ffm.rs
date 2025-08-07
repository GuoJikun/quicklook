use ez_ffmpeg::{FfmpegContext, FfmpegScheduler, Output};
use std::{path::PathBuf, thread};
use tiny_http::{Response, Server};

fn generate_hls(input_file: &str, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&output_dir)?;
    let m3u8_path = output_dir.join("playlist.m3u8");

    // 直接使用转码，确保Web兼容性和一致性
    println!("🔄 强制转码模式: 确保最大Web兼容性");

    // 编码器优先级：使用系统实际可用的编码器
    let video_codecs = vec![
        "h264",       // 系统默认H.264编码器 (通常可用)
        "mpeg4",      // MPEG-4编码器 (备选)
        "libvpx",     // VP8编码器 (如果可用)
        "libvpx-vp9", // VP9编码器 (如果可用)
    ];
    let audio_codecs = vec![
        "mp3",        // MP3编码器 (更兼容，优先)
        "aac",        // 系统默认AAC编码器
        "libmp3lame", // LAME MP3编码器 (如果可用)
    ];

    let mut last_error: Option<Box<dyn std::error::Error>> = None;

    for video_codec in &video_codecs {
        for audio_codec in &audio_codecs {
            // 使用 ez-ffmpeg 库配置 HLS 输出，支持边切边输出
            let mut output = Output::new(m3u8_path.to_str().unwrap())
                .set_format("hls")
                .set_video_codec(*video_codec)
                .set_audio_codec(*audio_codec)
                .set_format_opt("hls_time", "4") // 4秒一个切片，更短的延迟
                .set_format_opt("hls_list_size", "0") // 保留所有切片
                .set_format_opt("hls_flags", "independent_segments+append_list") // 移除split_by_time避免冲突
                .set_format_opt("hls_segment_type", "mpegts") // 明确指定段类型
                .set_format_opt("hls_base_url", "") // 设置基础URL
                .set_format_opt("force_key_frames", "expr:gte(t,n_forced*4)") // 强制关键帧每4秒
                .set_format_opt("avoid_negative_ts", "make_zero") // 避免负时间戳
                .set_format_opt("start_number", "0") // 从0开始编号
                .set_format_opt("hls_allow_cache", "1") // 允许缓存
                // 修复时间戳和持续时间问题
                .set_format_opt("max_muxing_queue_size", "1024") // 增加复用队列大小
                .set_format_opt("fflags", "+genpts+igndts") // 生成PTS并忽略DTS
                .set_format_opt("use_wallclock_as_timestamps", "1") // 使用时钟时间戳
                .set_format_opt("frame_drop_threshold", "0") // 禁用帧丢弃
                .set_format_opt(
                    "hls_segment_filename",
                    output_dir.join("segment_%03d.ts").to_str().unwrap(),
                );

            // 根据编码器类型设置简化的参数
            if video_codec.contains("libvpx") {
                // VP8/VP9 编码参数
                output = output
                    .set_video_codec_opt("deadline", "realtime") // 实时编码
                    .set_video_codec_opt("cpu-used", "8") // 最快速度
                    .set_video_codec_opt("crf", "28") // 质量控制
                    .set_video_codec_opt("g", "60"); // GOP大小
            } else if *video_codec == "mpeg4" {
                // MPEG-4 编码参数
                output = output
                    .set_video_codec_opt("qscale", "10") // 质量级别
                    .set_video_codec_opt("g", "60"); // GOP大小
            } else {
                // H.264 编码参数 - 使用最基本的参数，确保时间戳正确
                output = output
                    .set_video_codec_opt("b:v", "2000k") // 设置视频码率2Mbps
                    .set_video_codec_opt("crf", "28") // 质量控制
                    .set_video_codec_opt("g", "100") // GOP大小=帧率*4(25*4)
                    .set_video_codec_opt("bf", "0") // 禁用B帧，提高兼容性
                    .set_video_codec_opt("keyint_min", "25") // 最小关键帧间隔=帧率
                    .set_video_codec_opt("sc_threshold", "0") // 禁用场景切换检测
                    .set_video_codec_opt("r", "25") // 设置固定帧率25fps
                    .set_video_codec_opt("preset", "ultrafast") // 最快编码预设
                    .set_video_codec_opt("tune", "zerolatency") // 零延迟调优
                    .set_video_codec_opt("x264opts", "no-scenecut:force-cfr:filler") // x264特定选项
                    .set_video_codec_opt("video_track_timescale", "90000"); // 设置视频时间基准
            }

            // 音频编码参数 - 修复AAC编码器问题，确保音频时间戳正确
            output = output
                .set_audio_sample_rate(44100) // 44.1kHz采样率
                .set_audio_codec_opt("b", "128k"); // 音频码率

            // 对特定音频编码器设置额外参数
            if *audio_codec == "aac" {
                output = output
                    .set_audio_codec_opt("strict", "experimental") // 允许实验性AAC
                    .set_audio_codec_opt("ac", "2") // 使用ac参数指定立体声
                    .set_audio_codec_opt("profile", "aac_low") // 使用低复杂度AAC配置
                    .set_audio_codec_opt("ar", "44100"); // 明确指定采样率
            } else if *audio_codec == "mp3" || audio_codec.contains("mp3") {
                // MP3编码器参数
                output = output
                    .set_audio_channels(2) // 立体声
                    .set_audio_codec_opt("q", "4"); // MP3质量级别 (0-9, 4是好的平衡)
            } else {
                // 对其他音频编码器使用channels参数
                output = output.set_audio_channels(2); // 立体声
            }

            let context_result = FfmpegContext::builder()
                .input(input_file)
                .output(output)
                .build();

            match context_result {
                Ok(context) => {
                    println!("✅ 使用编码器: 视频={}, 音频={}", video_codec, audio_codec);

                    // 启动 ffmpeg，边切边输出（不等待完成）
                    match FfmpegScheduler::new(context).start() {
                        Ok(scheduler) => {
                            // 在后台线程中等待完成，不阻塞主线程
                            thread::spawn(move || match scheduler.wait() {
                                Ok(_) => println!("✅ HLS 转码完成"),
                                Err(e) => eprintln!("❌ HLS 转码错误: {:?}", e),
                            });
                            return Ok(());
                        }
                        Err(e) => {
                            last_error = Some(Box::new(e));
                            println!(
                                "⚠️ 编码器 {}+{} 启动失败，尝试下一个",
                                video_codec, audio_codec
                            );
                            continue;
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(Box::new(e));
                    println!(
                        "⚠️ 编码器 {}+{} 构建失败，尝试下一个",
                        video_codec, audio_codec
                    );
                    continue;
                }
            }
        }
    }

    // 如果所有编码器都失败了，返回最后一个错误
    Err(last_error.unwrap_or_else(|| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "所有编码器都不可用",
        ))
    }))
}

fn start_http_server(hls_dir: PathBuf) {
    thread::spawn(move || {
        let server = Server::http("127.0.0.1:17878").expect("Failed to start server");
        println!("✅ HLS server at http://127.0.0.1:17878");

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
                    }
                    Err(e) => {
                        eprintln!("Failed to read file {}: {}", full_path.display(), e);
                        let response =
                            Response::from_string("Internal Server Error").with_status_code(500);
                        let _ = request.respond(response);
                    }
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
    // 使用临时目录作为缓存目录
    let temp_dir = std::env::temp_dir();
    let output_dir = temp_dir.join("quicklook-hls-output");

    // 清理之前的文件
    if output_dir.exists() {
        let _ = std::fs::remove_dir_all(&output_dir);
    }

    // 启动 HTTP 服务（这里简化为多次调用会重复启动，生产可改为单例）
    start_http_server(output_dir.clone());

    // 启动 ffmpeg 线程，边切边生成
    let input_clone = input.clone();
    let output_dir_clone = output_dir.clone();
    std::thread::spawn(move || {
        // 等待一小段时间确保目录创建
        std::thread::sleep(std::time::Duration::from_millis(100));

        if let Err(e) = generate_hls(&input_clone, &output_dir_clone) {
            eprintln!("❌ ffmpeg error: {:?}", e);
        } else {
            println!("✅ HLS 转码启动成功，开始边切边输出");
        }
    });

    Ok(format!(
        "HLS 服务已启动，播放地址：http://127.0.0.1:17878/playlist.m3u8\n文件缓存位置：{}",
        output_dir.display()
    ))
}
