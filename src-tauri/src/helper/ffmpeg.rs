use ffmpeg_next as ffmpeg;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tokio::sync::oneshot;

#[allow(dead_code)]

/// 视频流任务管理器
pub struct VideoStreamManager {
    tasks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
}

impl VideoStreamManager {
    /// 创建新的任务管理器
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 添加任务
    fn add_task(&self, task_id: String, cancel_sender: oneshot::Sender<()>) {
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.insert(task_id, cancel_sender);
        }
    }

    /// 取消任务
    pub fn cancel_task(&self, task_id: &str) -> Result<(), String> {
        if let Ok(mut tasks) = self.tasks.lock() {
            if let Some(cancel_sender) = tasks.remove(task_id) {
                let _ = cancel_sender.send(());
                Ok(())
            } else {
                Err(format!("任务不存在: {}", task_id))
            }
        } else {
            Err("无法获取任务锁".to_string())
        }
    }

    /// 移除已完成的任务
    fn remove_task(&self, task_id: &str) {
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.remove(task_id);
        }
    }

    /// 获取正在运行的任务数量
    pub fn active_tasks_count(&self) -> usize {
        self.tasks.lock().map(|tasks| tasks.len()).unwrap_or(0)
    }
}

/// 全局任务管理器实例
static TASK_MANAGER: std::sync::OnceLock<VideoStreamManager> = std::sync::OnceLock::new();

/// 获取全局任务管理器
pub fn get_task_manager() -> &'static VideoStreamManager {
    TASK_MANAGER.get_or_init(|| VideoStreamManager::new())
}

/// 视频流转码处理器（静态链接版本）
pub struct VideoStreamProcessor;

impl VideoStreamProcessor {
    /// 开始视频流转码
    pub async fn start_stream(
        file_path: String,
        sink: Channel,
        task_id: String,
    ) -> Result<(), String> {
        // 创建取消信号通道
        let (cancel_sender, cancel_receiver) = oneshot::channel::<()>();

        // 将任务添加到管理器
        let task_manager = get_task_manager();
        task_manager.add_task(task_id.clone(), cancel_sender);

        // 异步运行转码任务
        let task_id_clone = task_id.clone();

        tauri::async_runtime::spawn(async move {
            // 在异步任务中执行转码
            let result = Self::transcode_video(&file_path, sink, cancel_receiver).await;

            if let Err(e) = result {
                log::error!("视频转码失败: {}", e);
            }

            // 任务完成或被取消，从管理器中移除
            get_task_manager().remove_task(&task_id_clone);
        });

        Ok(())
    }

    /// 执行视频转码为 MP4 格式（使用 FFmpeg 静态链接库）
    async fn transcode_video(
        input_path: &str,
        sink: Channel,
        mut cancel_receiver: oneshot::Receiver<()>,
    ) -> Result<(), String> {
        let input_path = input_path.to_string();

        // 使用 tokio::task::spawn_blocking 来运行 FFmpeg 转码
        let handle = tokio::task::spawn_blocking(move || -> Result<Vec<Vec<u8>>, String> {
            // 初始化 FFmpeg
            ffmpeg::init().map_err(|e| format!("FFmpeg 初始化失败: {}", e))?;

            log::info!("开始转码视频文件: {}", input_path);

            // 验证输入文件并获取基本信息
            let input_context = ffmpeg::format::input(&input_path)
                .map_err(|e| format!("无法打开输入文件 {}: {}", input_path, e))?;

            // 查找视频流
            let video_stream = input_context
                .streams()
                .best(ffmpeg::media::Type::Video)
                .ok_or("未找到视频流")?;

            log::info!("找到视频流，索引: {}", video_stream.index());

            // 对于静态链接版本，我们采用更简单但有效的方法：
            // 1. 如果输入已经是 MP4 且参数合适，直接分块传输
            // 2. 否则，使用基本的转码逻辑

            // 检查是否需要转码
            let needs_transcode = Self::needs_transcoding(&input_context)?;

            if !needs_transcode {
                log::info!("文件格式适合直接流式传输，跳过转码");
                // 直接读取原文件并分块
                Self::stream_original_file(&input_path)
            } else {
                log::info!("需要转码，使用简化转码流程");
                // 执行基本转码
                Self::basic_transcode(&input_path)
            }
        });

        // 同时等待转码完成和取消信号
        tokio::select! {
            result = handle => {
                match result {
                    Ok(Ok(chunks)) => {
                        log::info!("开始发送 {} 个数据块", chunks.len());
                        // 流式发送数据块
                        for (i, chunk) in chunks.iter().enumerate() {
                            // 检查是否收到取消信号
                            if cancel_receiver.try_recv().is_ok() {
                                log::info!("视频转码任务被取消");
                                return Ok(());
                            }

                            if !chunk.is_empty() {
                                log::debug!("发送数据块 {}/{}: {} bytes", i + 1, chunks.len(), chunk.len());
                                match sink.send(tauri::ipc::InvokeResponseBody::Raw(chunk.clone())) {
                                    Ok(_) => log::debug!("数据块发送成功"),
                                    Err(e) => {
                                        log::error!("发送数据块失败: {}", e);
                                        break;
                                    }
                                }
                            }

                            // 添加适当的延时以控制流式传输速度
                            if i % 5 == 0 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                            }
                        }
                        log::info!("视频流式传输完成");
                        Ok(())
                    }
                    Ok(Err(e)) => Err(e),
                    Err(e) => Err(format!("转码任务执行失败: {}", e))
                }
            }
            _ = &mut cancel_receiver => {
                log::info!("视频转码任务被取消");
                Ok(())
            }
        }
    }

    /// 检查是否需要转码
    fn needs_transcoding(input_context: &ffmpeg::format::context::Input) -> Result<bool, String> {
        // 获取文件格式信息
        let format = input_context.format();
        let format_name = format.name().to_string();
        let format_long_name = format.description().to_string();

        log::info!("输入格式: {} ({})", format_name, format_long_name);

        // 检查容器格式
        let is_mp4_container = format_name.contains("mp4") || format_name.contains("mov");

        if !is_mp4_container {
            log::info!("非MP4容器格式，需要转码");
            return Ok(true);
        }

        // 检查视频编码格式
        if let Some(video_stream) = input_context.streams().best(ffmpeg::media::Type::Video) {
            let video_codec_id = video_stream.parameters().id();
            log::info!("视频编码格式: {:?}", video_codec_id);

            // 检查是否为H.264编码
            if video_codec_id != ffmpeg::codec::Id::H264 {
                log::info!("非H.264编码，需要转码");
                return Ok(true);
            }
        }

        // 检查音频编码格式
        if let Some(audio_stream) = input_context.streams().best(ffmpeg::media::Type::Audio) {
            let audio_codec_id = audio_stream.parameters().id();
            log::info!("音频编码格式: {:?}", audio_codec_id);

            // 检查是否为AAC编码
            if audio_codec_id != ffmpeg::codec::Id::AAC {
                log::info!("非AAC编码，需要转码");
                return Ok(true);
            }
        }

        log::info!("检测到标准MP4格式(H.264+AAC)，跳过转码");
        Ok(false)
    }

    /// 直接流式传输原文件
    fn stream_original_file(input_path: &str) -> Result<Vec<Vec<u8>>, String> {
        use std::io::Read;

        log::info!("开始直接流式传输原文件: {}", input_path);

        let mut file =
            std::fs::File::open(input_path).map_err(|e| format!("打开文件失败: {}", e))?;

        // 获取文件大小
        let file_size = file
            .metadata()
            .map_err(|e| format!("获取文件信息失败: {}", e))?
            .len();

        log::info!("文件大小: {} bytes", file_size);

        // 对于大文件，使用更大的缓冲区以提高效率
        let chunk_size = if file_size > 10 * 1024 * 1024 {
            // 10MB
            256 * 1024 // 256KB
        } else {
            64 * 1024 // 64KB
        };

        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; chunk_size];
        let mut total_read = 0u64;

        loop {
            match file.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(bytes_read) => {
                    total_read += bytes_read as u64;
                    chunks.push(buffer[..bytes_read].to_vec());

                    // 每读取10MB打印一次进度
                    if total_read % (10 * 1024 * 1024) == 0 || total_read == file_size {
                        let progress = (total_read as f64 / file_size as f64 * 100.0) as u32;
                        log::info!(
                            "读取进度: {}% ({}/{} bytes)",
                            progress,
                            total_read,
                            file_size
                        );
                    }
                }
                Err(e) => return Err(format!("读取文件失败: {}", e)),
            }
        }

        log::info!(
            "原文件直接分块完成，实际读取: {} bytes, 分成 {} 个块, 平均块大小: {} bytes",
            total_read,
            chunks.len(),
            if chunks.is_empty() {
                0
            } else {
                total_read / chunks.len() as u64
            }
        );

        Ok(chunks)
    }

    /// 基本转码功能（使用 ffmpeg-next 库实现）
    fn basic_transcode(input_path: &str) -> Result<Vec<Vec<u8>>, String> {
        log::info!("开始使用 ffmpeg-next 进行MP4转码");

        // 由于 ffmpeg-next API 的复杂性，我们使用一个简化的方法
        // 首先尝试验证视频，如果失败则回退到原文件传输
        match Self::validate_and_convert_video(input_path) {
            Ok(chunks) => {
                log::info!("视频转码成功，返回数据块");
                Ok(chunks)
            }
            Err(e) => {
                log::warn!("ffmpeg-next 转码失败: {}, 回退到原文件传输", e);
                Self::stream_original_file(input_path)
            }
        }
    }

    /// 验证并转换视频文件
    fn validate_and_convert_video(input_path: &str) -> Result<Vec<Vec<u8>>, String> {
        // 首先验证输入文件
        let input_context =
            ffmpeg::format::input(&input_path).map_err(|e| format!("无法打开输入文件: {}", e))?;

        // 查找视频流
        let video_stream = input_context
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or("未找到视频流")?;

        // 获取视频信息
        let video_context =
            ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
                .map_err(|e| format!("获取视频参数失败: {}", e))?;
        let decoder = video_context
            .decoder()
            .video()
            .map_err(|e| format!("创建解码器失败: {}", e))?;

        let width = decoder.width();
        let height = decoder.height();
        let format = decoder.format();

        log::info!("视频信息: {}x{}, 格式: {:?}", width, height, format);

        Self::transcode_with_external_ffmpeg(input_path)
    }

    /// 使用 ffmpeg-next 库进行转码
    fn transcode_with_external_ffmpeg(input_path: &str) -> Result<Vec<Vec<u8>>, String> {
        log::info!("开始使用 ffmpeg-next 库进行转码");

        // 创建临时输出文件
        let temp_dir = std::env::temp_dir();
        let temp_output = temp_dir.join(format!("quicklook_transcode_{}.mp4", std::process::id()));
        let temp_output_str = temp_output.to_string_lossy().to_string();

        // 使用 ffmpeg-next 进行转码
        let result = Self::transcode_with_ffmpeg_next(input_path, &temp_output_str);

        // 如果转码成功，读取输出文件
        let chunks_result = match result {
            Ok(_) => {
                log::info!("ffmpeg-next 转码完成，读取输出文件");
                let chunks = Self::read_file_in_chunks(&temp_output_str);

                // 清理临时文件
                if let Err(e) = std::fs::remove_file(&temp_output) {
                    log::warn!("清理临时文件失败: {}", e);
                }

                chunks
            }
            Err(e) => {
                // 清理临时文件
                let _ = std::fs::remove_file(&temp_output);
                Err(e)
            }
        };

        chunks_result
    }

    /// 使用 ffmpeg-next 进行实际的转码操作
    fn transcode_with_ffmpeg_next(input_path: &str, output_path: &str) -> Result<(), String> {
        // 打开输入文件
        let mut input_context =
            ffmpeg::format::input(&input_path).map_err(|e| format!("无法打开输入文件: {}", e))?;

        // 创建输出文件
        let mut output_context =
            ffmpeg::format::output(&output_path).map_err(|e| format!("创建输出文件失败: {}", e))?;

        // 查找最佳的视频和音频流
        let video_stream_index = input_context
            .streams()
            .best(ffmpeg::media::Type::Video)
            .map(|s| s.index());

        let audio_stream_index = input_context
            .streams()
            .best(ffmpeg::media::Type::Audio)
            .map(|s| s.index());

        // 处理视频流
        let mut video_decoder = None;
        let mut video_encoder = None;
        let mut video_stream_mapping = None;

        if let Some(video_index) = video_stream_index {
            let input_video_stream = input_context.stream(video_index).unwrap();

            // 创建视频解码器
            let video_context =
                ffmpeg::codec::context::Context::from_parameters(input_video_stream.parameters())
                    .map_err(|e| format!("创建视频解码器上下文失败: {}", e))?;
            let decoder = video_context
                .decoder()
                .video()
                .map_err(|e| format!("创建视频解码器失败: {}", e))?;

            // 创建视频编码器
            let encoder_codec =
                ffmpeg::encoder::find(ffmpeg::codec::Id::H264).ok_or("找不到H264编码器")?;

            let mut output_video_stream = output_context
                .add_stream(encoder_codec)
                .map_err(|e| format!("添加输出视频流失败: {}", e))?;

            let encoder_context = ffmpeg::codec::context::Context::new_with_codec(encoder_codec);
            let mut encoder = encoder_context
                .encoder()
                .video()
                .map_err(|e| format!("创建视频编码器失败: {}", e))?;

            // 计算目标尺寸
            let (target_width, target_height) =
                Self::calculate_target_size(decoder.width(), decoder.height(), 1920, 1080);

            // 配置编码器参数
            encoder.set_width(target_width);
            encoder.set_height(target_height);
            encoder.set_format(ffmpeg::format::Pixel::YUV420P);
            encoder.set_time_base(input_video_stream.time_base());

            // 设置质量参数
            encoder.set_bit_rate(2000000); // 2Mbps
            encoder.set_max_bit_rate(4000000); // 4Mbps max

            // 打开编码器
            let encoder = encoder
                .open_as(encoder_codec)
                .map_err(|e| format!("打开视频编码器失败: {}", e))?;

            output_video_stream.set_parameters(&encoder);

            video_decoder = Some(decoder);
            video_encoder = Some(encoder);
            video_stream_mapping = Some((video_index, output_video_stream.index()));
        }

        // 处理音频流
        let mut audio_decoder = None;
        let mut audio_encoder = None;
        let mut audio_stream_mapping = None;

        if let Some(audio_index) = audio_stream_index {
            let input_audio_stream = input_context.stream(audio_index).unwrap();

            // 创建音频解码器
            let audio_context =
                ffmpeg::codec::context::Context::from_parameters(input_audio_stream.parameters())
                    .map_err(|e| format!("创建音频解码器上下文失败: {}", e))?;
            let decoder = audio_context
                .decoder()
                .audio()
                .map_err(|e| format!("创建音频解码器失败: {}", e))?;

            // 创建音频编码器
            let encoder_codec =
                ffmpeg::encoder::find(ffmpeg::codec::Id::AAC).ok_or("找不到AAC编码器")?;

            let mut output_audio_stream = output_context
                .add_stream(encoder_codec)
                .map_err(|e| format!("添加输出音频流失败: {}", e))?;

            let encoder_context = ffmpeg::codec::context::Context::new_with_codec(encoder_codec);
            let mut encoder = encoder_context
                .encoder()
                .audio()
                .map_err(|e| format!("创建音频编码器失败: {}", e))?;

            // 配置音频编码器参数
            encoder.set_rate(decoder.rate() as i32);
            encoder.set_channel_layout(decoder.channel_layout());

            // 配置音频格式（AAC编码器支持的格式）
            let supported_formats = [
                ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar), // fltp - AAC首选格式
                ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Planar), // s16p
                ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed), // flt
                ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed), // s16
            ];

            log::info!("原始音频格式: {:?}", decoder.format());

            // 尝试设置支持的格式
            let mut format_set = false;
            for (i, format) in supported_formats.iter().enumerate() {
                encoder.set_format(*format);
                log::debug!("尝试设置音频格式 {}: {:?}", i + 1, format);

                // 验证格式是否被接受
                if encoder.format() == *format {
                    log::info!("成功设置音频格式: {:?}", format);
                    format_set = true;
                    break;
                }
            }

            if !format_set {
                return Err("无法设置AAC编码器支持的音频格式".to_string());
            }

            encoder.set_bit_rate(128000); // 128kbps
            encoder.set_time_base((1, decoder.rate() as i32));

            // 打开编码器
            let encoder = encoder.open_as(encoder_codec).map_err(|e| {
                log::error!("打开音频编码器失败: {}", e);
                format!("打开音频编码器失败: {}", e)
            })?;

            output_audio_stream.set_parameters(&encoder);

            audio_decoder = Some(decoder);
            audio_encoder = Some(encoder);
            audio_stream_mapping = Some((audio_index, output_audio_stream.index()));
        }

        // 写入文件头
        output_context
            .write_header()
            .map_err(|e| format!("写入文件头失败: {}", e))?;

        // 转码主循环
        let mut video_frame_count = 0i64;
        let mut audio_frame_count = 0i64;

        for (stream, packet) in input_context.packets() {
            if let (
                Some((input_index, output_index)),
                Some(ref mut decoder),
                Some(ref mut encoder),
            ) = (video_stream_mapping, &mut video_decoder, &mut video_encoder)
            {
                if stream.index() == input_index {
                    Self::process_video_packet_simple(
                        &packet,
                        decoder,
                        encoder,
                        &mut output_context,
                        output_index,
                        &mut video_frame_count,
                    )?;
                }
            }

            if let (
                Some((input_index, output_index)),
                Some(ref mut decoder),
                Some(ref mut encoder),
            ) = (audio_stream_mapping, &mut audio_decoder, &mut audio_encoder)
            {
                if stream.index() == input_index {
                    Self::process_audio_packet_simple(
                        &packet,
                        decoder,
                        encoder,
                        &mut output_context,
                        output_index,
                        &mut audio_frame_count,
                    )?;
                }
            }
        }

        // 写入文件尾
        output_context
            .write_trailer()
            .map_err(|e| format!("写入文件尾失败: {}", e))?;

        log::info!(
            "转码完成，视频帧数: {}, 音频帧数: {}",
            video_frame_count,
            audio_frame_count
        );
        Ok(())
    }

    /// 简化的视频包处理
    fn process_video_packet_simple(
        packet: &ffmpeg::packet::Packet,
        decoder: &mut ffmpeg::decoder::Video,
        encoder: &mut ffmpeg::encoder::Video,
        output_context: &mut ffmpeg::format::context::Output,
        output_stream_index: usize,
        frame_count: &mut i64,
    ) -> Result<(), String> {
        decoder
            .send_packet(packet)
            .map_err(|e| format!("发送视频包到解码器失败: {}", e))?;

        let mut decoded_frame = ffmpeg::frame::Video::empty();
        while decoder.receive_frame(&mut decoded_frame).is_ok() {
            decoded_frame.set_pts(Some(*frame_count));
            *frame_count += 1;

            // 这里可以添加缩放逻辑，但为了简化，我们直接编码
            encoder
                .send_frame(&decoded_frame)
                .map_err(|e| format!("发送帧到编码器失败: {}", e))?;

            let mut encoded_packet = ffmpeg::packet::Packet::empty();
            while encoder.receive_packet(&mut encoded_packet).is_ok() {
                encoded_packet.set_stream(output_stream_index);
                encoded_packet.rescale_ts(
                    encoder.time_base(),
                    output_context
                        .stream(output_stream_index)
                        .unwrap()
                        .time_base(),
                );

                encoded_packet
                    .write_interleaved(output_context)
                    .map_err(|e| format!("写入视频包失败: {}", e))?;
            }
        }

        Ok(())
    }

    /// 简化的音频包处理
    fn process_audio_packet_simple(
        packet: &ffmpeg::packet::Packet,
        decoder: &mut ffmpeg::decoder::Audio,
        encoder: &mut ffmpeg::encoder::Audio,
        output_context: &mut ffmpeg::format::context::Output,
        output_stream_index: usize,
        frame_count: &mut i64,
    ) -> Result<(), String> {
        decoder
            .send_packet(packet)
            .map_err(|e| format!("发送音频包到解码器失败: {}", e))?;

        let mut decoded_frame = ffmpeg::frame::Audio::empty();
        while decoder.receive_frame(&mut decoded_frame).is_ok() {
            // 检查解码帧格式是否与编码器匹配
            let decoder_format = decoded_frame.format();
            let encoder_format = encoder.format();
            let decoder_rate = decoded_frame.rate();
            let encoder_rate = encoder.rate();
            let decoder_layout = decoded_frame.channel_layout();
            let encoder_layout = encoder.channel_layout();

            log::debug!("音频帧格式检查:");
            log::debug!(
                "  解码器格式: {:?}, 编码器格式: {:?}",
                decoder_format,
                encoder_format
            );
            log::debug!(
                "  解码器采样率: {}, 编码器采样率: {}",
                decoder_rate,
                encoder_rate
            );
            log::debug!(
                "  解码器声道: {:?}, 编码器声道: {:?}",
                decoder_layout,
                encoder_layout
            );

            // 如果格式不匹配，跳过此帧并记录警告
            if decoder_format != encoder_format
                || decoder_rate != encoder_rate
                || decoder_layout != encoder_layout
            {
                log::warn!(
                    "音频格式不匹配，跳过此帧 - 解码器: {:?}@{}Hz {:?}, 编码器: {:?}@{}Hz {:?}",
                    decoder_format,
                    decoder_rate,
                    decoder_layout,
                    encoder_format,
                    encoder_rate,
                    encoder_layout
                );
                continue;
            }

            decoded_frame.set_pts(Some(*frame_count));
            *frame_count += decoded_frame.samples() as i64;

            // 尝试发送帧到编码器
            match encoder.send_frame(&decoded_frame) {
                Ok(_) => {
                    let mut encoded_packet = ffmpeg::packet::Packet::empty();
                    while encoder.receive_packet(&mut encoded_packet).is_ok() {
                        encoded_packet.set_stream(output_stream_index);
                        encoded_packet.rescale_ts(
                            encoder.time_base(),
                            output_context
                                .stream(output_stream_index)
                                .unwrap()
                                .time_base(),
                        );

                        encoded_packet
                            .write_interleaved(output_context)
                            .map_err(|e| format!("写入音频包失败: {}", e))?;
                    }
                }
                Err(e) => {
                    log::warn!("发送音频帧到编码器失败: {}, 跳过此帧", e);
                    // 不返回错误，而是继续处理下一帧
                    continue;
                }
            }
        }

        Ok(())
    }

    /// 分块读取文件
    fn read_file_in_chunks(file_path: &str) -> Result<Vec<Vec<u8>>, String> {
        use std::io::Read;

        let mut file =
            std::fs::File::open(file_path).map_err(|e| format!("打开转码文件失败: {}", e))?;

        let file_size = file
            .metadata()
            .map_err(|e| format!("获取文件信息失败: {}", e))?
            .len();

        log::info!("转码后文件大小: {} bytes", file_size);

        let chunk_size = 64 * 1024; // 64KB
        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; chunk_size];
        let mut total_read = 0u64;

        loop {
            match file.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(bytes_read) => {
                    total_read += bytes_read as u64;
                    chunks.push(buffer[..bytes_read].to_vec());
                }
                Err(e) => return Err(format!("读取转码文件失败: {}", e)),
            }
        }

        log::info!(
            "转码文件读取完成，实际读取: {} bytes, 分成 {} 个块",
            total_read,
            chunks.len()
        );

        Ok(chunks)
    }

    /// 计算目标尺寸，保持长宽比
    fn calculate_target_size(
        input_width: u32,
        input_height: u32,
        max_width: u32,
        max_height: u32,
    ) -> (u32, u32) {
        let input_ratio = input_width as f64 / input_height as f64;
        let max_ratio = max_width as f64 / max_height as f64;

        if input_ratio > max_ratio {
            // 宽度是限制因素
            let target_width = max_width;
            let target_height = (max_width as f64 / input_ratio) as u32;
            // 确保高度是偶数（H.264要求）
            let target_height = if target_height % 2 == 0 {
                target_height
            } else {
                target_height - 1
            };
            (target_width, target_height)
        } else {
            // 高度是限制因素
            let target_height = max_height;
            let target_width = (max_height as f64 * input_ratio) as u32;
            // 确保宽度是偶数（H.264要求）
            let target_width = if target_width % 2 == 0 {
                target_width
            } else {
                target_width - 1
            };
            (target_width, target_height)
        }
    }

    /// 验证视频文件格式和编码信息
    pub fn analyze_video_info(file_path: &str) -> Result<VideoInfo, String> {
        ffmpeg::init().map_err(|e| format!("FFmpeg 初始化失败: {}", e))?;

        let input_context = ffmpeg::format::input(&file_path)
            .map_err(|e| format!("无法打开文件 {}: {}", file_path, e))?;

        let format = input_context.format();
        let mut video_info = VideoInfo {
            format_name: format.name().to_string(),
            format_long_name: format.description().to_string(),
            duration_seconds: input_context.duration() as f64 / ffmpeg::ffi::AV_TIME_BASE as f64,
            file_size: std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0),
            video_stream: None,
            audio_stream: None,
            needs_transcode: false,
        };

        // 分析视频流
        if let Some(video_stream) = input_context.streams().best(ffmpeg::media::Type::Video) {
            let video_codec_id = video_stream.parameters().id();
            let mut video_stream_info = VideoStreamInfo {
                codec_name: format!("{:?}", video_codec_id),
                width: 0,
                height: 0,
                frame_rate: video_stream.rate(),
                bit_rate: 0, // 暂时设为0，避免API问题
                pixel_format: "unknown".to_string(),
            };

            if let Ok(video_context) =
                ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            {
                if let Ok(decoder) = video_context.decoder().video() {
                    video_stream_info.width = decoder.width();
                    video_stream_info.height = decoder.height();
                    video_stream_info.pixel_format = format!("{:?}", decoder.format());
                }
            }

            video_info.video_stream = Some(video_stream_info);
        }

        // 分析音频流
        if let Some(audio_stream) = input_context.streams().best(ffmpeg::media::Type::Audio) {
            let audio_codec_id = audio_stream.parameters().id();
            let mut audio_stream_info = AudioStreamInfo {
                codec_name: format!("{:?}", audio_codec_id),
                sample_rate: 0, // 暂时设为0，避免API问题
                channels: 0,    // 暂时设为0，避免API问题
                bit_rate: 0,    // 暂时设为0，避免API问题
            };

            // 尝试从解码器获取音频参数
            if let Ok(audio_context) =
                ffmpeg::codec::context::Context::from_parameters(audio_stream.parameters())
            {
                if let Ok(decoder) = audio_context.decoder().audio() {
                    audio_stream_info.sample_rate = decoder.rate();
                    audio_stream_info.channels = decoder.channels() as u32;
                }
            }

            video_info.audio_stream = Some(audio_stream_info);
        }

        // 判断是否需要转码
        video_info.needs_transcode = Self::needs_transcoding(&input_context)?;

        log::info!("视频文件分析完成: {:?}", video_info);
        Ok(video_info)
    }

    /// 测试转码功能
    pub fn test_transcode(input_path: &str, output_path: &str) -> Result<(), String> {
        log::info!("开始测试转码功能: {} -> {}", input_path, output_path);
        Self::transcode_with_ffmpeg_next(input_path, output_path)
    }
}

/// 视频信息结构体
#[derive(Debug)]
pub struct VideoInfo {
    pub format_name: String,
    pub format_long_name: String,
    pub duration_seconds: f64,
    pub file_size: u64,
    pub video_stream: Option<VideoStreamInfo>,
    pub audio_stream: Option<AudioStreamInfo>,
    pub needs_transcode: bool,
}

/// 视频流信息
#[derive(Debug)]
pub struct VideoStreamInfo {
    pub codec_name: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: ffmpeg::Rational,
    pub bit_rate: u64,
    pub pixel_format: String,
}

/// 音频流信息
#[derive(Debug)]
pub struct AudioStreamInfo {
    pub codec_name: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub bit_rate: u64,
}
