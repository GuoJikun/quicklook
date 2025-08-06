use ez_ffmpeg::{FfmpegContext, FfmpegScheduler, Output};
use std::{path::PathBuf, thread};
use tiny_http::{Response, Server};

fn generate_hls(input_file: &str, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&output_dir)?;
    let m3u8_path = output_dir.join("playlist.m3u8");

    // 使用 ez-ffmpeg 库配置 HLS 输出，支持边切边输出
    let output = Output::new(m3u8_path.to_str().unwrap())
        .set_format("hls")
        .set_video_codec("libx264")
        .set_audio_codec("aac")
        .set_format_opt("hls_time", "4") // 4秒一个切片，更短的延迟
        .set_format_opt("hls_list_size", "0") // 保留所有切片
        .set_format_opt("hls_flags", "independent_segments+append_list") // 支持实时追加
        .set_format_opt(
            "hls_segment_filename",
            output_dir.join("segment_%03d.ts").to_str().unwrap(),
        );

    let context = FfmpegContext::builder()
        .input(input_file)
        .output(output)
        .build()?;

    // 启动 ffmpeg，边切边输出（不等待完成）
    let scheduler = FfmpegScheduler::new(context).start()?;

    // 在后台线程中等待完成，不阻塞主线程
    thread::spawn(move || match scheduler.wait() {
        Ok(_) => println!("✅ HLS 转码完成"),
        Err(e) => eprintln!("❌ HLS 转码错误: {:?}", e),
    });

    Ok(())
}

fn start_http_server(hls_dir: PathBuf) {
    thread::spawn(move || {
        let server = Server::http("127.0.0.1:7878").expect("Failed to start server");
        println!("✅ HLS server at http://127.0.0.1:7878");

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
        "HLS 服务已启动，播放地址：http://127.0.0.1:7878/playlist.m3u8\n文件缓存位置：{}",
        output_dir.display()
    ))
}
