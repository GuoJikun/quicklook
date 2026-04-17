use app_lib::helper;
use std::{env, thread, time::Duration}; // 使用库名引用公开模块

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: cargo run --bin hls_probe -- <video_path> [wait_seconds]");
        return;
    }
    let video_path = &args[1];
    let wait_secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(12);

    println!(
        "[hls_probe] 启动 HLS 转码: {} (等待 {}s)",
        video_path, wait_secs
    );
    match helper::ffm::start_hls_process(video_path.clone()) {
        Ok(msg) => println!("[hls_probe] start_hls_process 返回: \n{}", msg),
        Err(e) => {
            eprintln!("启动失败: {}", e);
            return;
        }
    }

    thread::sleep(Duration::from_secs(wait_secs));

    let temp_dir = std::env::temp_dir().join("quicklook-hls-output");
    let playlist = temp_dir.join("playlist.m3u8");
    println!("[hls_probe] 期待播放列表位置: {}", playlist.display());
    if playlist.exists() {
        match std::fs::read_to_string(&playlist) {
            Ok(content) => {
                println!("[hls_probe] playlist.m3u8 前 25 行:");
                for (i, line) in content.lines().take(25).enumerate() {
                    println!("{:02}: {}", i + 1, line);
                }
            }
            Err(e) => eprintln!("读取播放列表失败: {}", e),
        }
    } else {
        eprintln!("未找到 playlist.m3u8，可能转码仍在进行或启动失败。");
    }

    println!("[hls_probe] 结束。可增加等待时间再次运行观察。");
}
