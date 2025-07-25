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
            // 在异步任务中执行处理
            let result = Self::process_video(&file_path, sink, cancel_receiver).await;

            if let Err(e) = result {
                log::error!("视频处理失败: {}", e);
            }

            // 任务完成或被取消，从管理器中移除
            get_task_manager().remove_task(&task_id_clone);
        });

        Ok(())
    }

    /// 处理视频文件（根据是否需要转码选择不同的处理方式）
    async fn process_video(
        input_path: &str,
        sink: Channel,
        mut cancel_receiver: oneshot::Receiver<()>,
    ) -> Result<(), String> {
        let input_path = input_path.to_string();

        // 使用 tokio::task::spawn_blocking 来运行 FFmpeg 处理
        let handle = tokio::task::spawn_blocking(move || -> Result<Vec<Vec<u8>>, String> {
            // 初始化 FFmpeg
            ffmpeg::init().map_err(|e| format!("FFmpeg 初始化失败: {}", e))?;

            log::info!("开始处理视频文件: {}", input_path);

            // 验证输入文件并获取基本信息
            let input_context = ffmpeg::format::input(&input_path)
                .map_err(|e| format!("无法打开输入文件 {}: {}", input_path, e))?;

            // 查找视频流
            let _video_stream = input_context
                .streams()
                .best(ffmpeg::media::Type::Video)
                .ok_or("未找到视频流")?;

            // 检查是否需要转码
            let needs_transcode = Self::needs_transcoding(&input_context)?;

            if needs_transcode {
                log::info!("需要转码，使用转码流程");
                Self::transcode(&input_path)
            } else {
                log::info!("文件格式适合直接流式传输，跳过转码");
                Self::stream_original_file(&input_path)
            }
        });

        // 同时等待处理完成和取消信号
        tokio::select! {
            result = handle => {
                match result {
                    Ok(Ok(chunks)) => {
                        log::info!("开始发送 {} 个数据块", chunks.len());
                        // 流式发送数据块
                        for (i, chunk) in chunks.iter().enumerate() {
                            // 检查是否收到取消信号
                            if cancel_receiver.try_recv().is_ok() {
                                log::info!("视频处理任务被取消");
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
                    Err(e) => Err(format!("处理任务执行失败: {}", e))
                }
            }
            _ = &mut cancel_receiver => {
                log::info!("视频处理任务被取消");
                Ok(())
            }
        }
    }

    /// 检查是否是 MP4 格式
    fn needs_transcoding(input_context: &ffmpeg::format::context::Input) -> Result<bool, String> {
        // 获取文件格式信息
        let format = input_context.format();
        let format_name = format.name().to_string();
        let format_long_name = format.description().to_string();

        log::info!("输入格式: {} ({})", format_name, format_long_name);

        // 检查容器格式是否为MP4
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

    /// 将视频转码为MP4格式并作为流返回给前端
    fn transcode(input_path: &str) -> Result<Vec<Vec<u8>>, String> {
        log::info!("开始转码视频文件为MP4格式: {}", input_path);

        // 创建临时输出文件
        let temp_dir = std::env::temp_dir();
        let temp_output = temp_dir.join(format!("quicklook_transcode_{}.mp4", std::process::id()));
        let temp_output_str = temp_output.to_string_lossy().to_string();

        // 使用 ffmpeg-next 进行转码
        let result = Self::transcode_with_ffmpeg_next(input_path, &temp_output_str);

        // 如果转码成功，读取输出文件
        let chunks_result = match result {
            Ok(_) => {
                log::info!("转码完成，读取输出文件");
                let chunks = Self::read_file_in_chunks(&temp_output_str);

                // 清理临时文件
                if let Err(e) = std::fs::remove_file(&temp_output) {
                    log::warn!("清理临时文件失败: {}", e);
                }

                chunks
            }
            Err(e) => {
                log::error!("转码失败: {}, 回退到直接传输原文件", e);
                // 清理临时文件
                let _ = std::fs::remove_file(&temp_output);

                // 回退到直接传输原文件
                Self::stream_original_file(input_path)
            }
        };

        chunks_result
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

    /// 查找可用的视频编码器
    fn find_available_video_encoder() -> Result<ffmpeg::codec::Id, String> {
        // 直接使用libx264编码器
        if let Some(_encoder) = ffmpeg::encoder::find_by_name("libx264") {
            log::info!("找到可用的视频编码器: libx264");
            return Ok(ffmpeg::codec::Id::H264);
        }

        // 如果libx264不可用，按优先级顺序尝试其他编码器
        let encoder_ids = [
            ffmpeg::codec::Id::MPEG4, // MPEG-4备选
            ffmpeg::codec::Id::H265,  // H.265/HEVC
            ffmpeg::codec::Id::VP8,   // VP8
            ffmpeg::codec::Id::VP9,   // VP9
        ];

        for codec_id in encoder_ids.iter() {
            if let Some(_encoder) = ffmpeg::encoder::find(*codec_id) {
                log::info!("找到可用的视频编码器: {:?}", codec_id);
                return Ok(*codec_id);
            } else {
                log::debug!("视频编码器不可用: {:?}", codec_id);
            }
        }

        Err("未找到任何可用的视频编码器".to_string())
    }

    /// 查找可用的音频编码器
    fn find_available_audio_encoder() -> Result<ffmpeg::codec::Id, String> {
        // 按优先级顺序尝试不同的音频编码器
        let encoder_ids = [
            ffmpeg::codec::Id::AAC,    // 首选AAC
            ffmpeg::codec::Id::MP3,    // MP3备选
            ffmpeg::codec::Id::VORBIS, // Vorbis
            ffmpeg::codec::Id::OPUS,   // Opus
        ];

        for codec_id in encoder_ids.iter() {
            if let Some(_encoder) = ffmpeg::encoder::find(*codec_id) {
                log::info!("找到可用的音频编码器: {:?}", codec_id);
                return Ok(*codec_id);
            } else {
                log::debug!("音频编码器不可用: {:?}", codec_id);
            }
        }

        Err("未找到任何可用的音频编码器".to_string())
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

            // 尝试查找可用的视频编码器
            let encoder_codec_id = Self::find_available_video_encoder()?;

            // 如果是H.264，直接使用libx264编码器
            let encoder_codec = if encoder_codec_id == ffmpeg::codec::Id::H264 {
                ffmpeg::encoder::find_by_name("libx264")
                    .ok_or("无法找到libx264编码器".to_string())?
            } else {
                ffmpeg::encoder::find(encoder_codec_id)
                    .ok_or(format!("无法找到视频编码器: {:?}", encoder_codec_id))?
            };

            log::info!(
                "使用视频编码器: {:?} ({})",
                encoder_codec_id,
                encoder_codec.name()
            );

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

            // 根据编码器类型设置像素格式
            if encoder_codec_id == ffmpeg::codec::Id::H264 {
                // 直接使用libx264，设置YUV420P像素格式
                log::info!("使用 libx264 编码器，设置 YUV420P 像素格式");
                encoder.set_format(ffmpeg::format::Pixel::YUV420P);
            } else {
                // 对于非H.264编码器，使用默认格式
                encoder.set_format(ffmpeg::format::Pixel::YUV420P);
            }

            // 根据编码器类型设置合适的时间基
            let input_time_base = input_video_stream.time_base();
            match encoder_codec_id {
                ffmpeg::codec::Id::MPEG4 => {
                    // MPEG4标准要求时间基分母不超过65535
                    // 检查输入时间基是否符合MPEG4标准
                    if input_time_base.1 > 65535 {
                        encoder.set_time_base((1, 30)); // 使用30fps标准时间基
                        log::info!(
                            "输入时间基 {}/{} 超过MPEG4限制，使用标准时间基: 1/30",
                            input_time_base.0,
                            input_time_base.1
                        );
                    } else {
                        encoder.set_time_base(input_time_base);
                        log::info!(
                            "使用输入时间基: {}/{}",
                            input_time_base.0,
                            input_time_base.1
                        );
                    }
                }
                _ => {
                    // 其他编码器使用输入时间基
                    encoder.set_time_base(input_time_base);
                    log::info!(
                        "使用输入时间基: {}/{}",
                        input_time_base.0,
                        input_time_base.1
                    );
                }
            }

            // 设置质量参数
            encoder.set_bit_rate(2000000); // 2Mbps
            encoder.set_max_bit_rate(4000000); // 4Mbps max

            // 尝试打开编码器，如果失败则尝试降低质量设置
            let encoder = match encoder.open_as(encoder_codec) {
                Ok(enc) => enc,
                Err(e) => {
                    log::warn!("使用默认设置打开编码器失败: {}, 尝试简化设置", e);

                    // 重新创建编码器并使用更保守的设置
                    let encoder_context =
                        ffmpeg::codec::context::Context::new_with_codec(encoder_codec);
                    let mut encoder = encoder_context
                        .encoder()
                        .video()
                        .map_err(|e| format!("重新创建视频编码器失败: {}", e))?;

                    // 使用更保守的设置
                    encoder.set_width(target_width);
                    encoder.set_height(target_height);

                    // 在重试时也要设置正确的像素格式
                    if encoder_codec_id == ffmpeg::codec::Id::H264 {
                        // 使用libx264，设置YUV420P像素格式
                        encoder.set_format(ffmpeg::format::Pixel::YUV420P);
                    } else {
                        encoder.set_format(ffmpeg::format::Pixel::YUV420P);
                    }

                    // 根据编码器类型设置合适的时间基
                    match encoder_codec_id {
                        ffmpeg::codec::Id::MPEG4 => {
                            // MPEG4标准要求时间基分母不超过65535
                            encoder.set_time_base((1, 30)); // 30fps标准时间基
                            log::info!("为MPEG4编码器设置时间基: 1/30");
                        }
                        ffmpeg::codec::Id::H264 => {
                            encoder.set_time_base((1, 25)); // H.264常用时间基
                            log::info!("为H.264编码器设置时间基: 1/25");
                        }
                        ffmpeg::codec::Id::H265 => {
                            encoder.set_time_base((1, 25)); // H.265常用时间基
                            log::info!("为H.265编码器设置时间基: 1/25");
                        }
                        _ => {
                            // 其他编码器使用通用时间基
                            encoder.set_time_base((1, 25));
                            log::info!("为编码器 {:?} 设置通用时间基: 1/25", encoder_codec_id);
                        }
                    }

                    encoder.set_bit_rate(1000000); // 降低码率到1Mbps

                    encoder
                        .open_as(encoder_codec)
                        .map_err(|e| format!("使用简化设置打开视频编码器失败: {}", e))?
                }
            };

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

            // 尝试查找可用的音频编码器
            let encoder_codec_id = Self::find_available_audio_encoder()?;
            let encoder_codec = ffmpeg::encoder::find(encoder_codec_id)
                .ok_or(format!("无法找到音频编码器: {:?}", encoder_codec_id))?;

            log::info!("使用音频编码器: {:?}", encoder_codec_id);

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

            // 根据编码器类型设置音频格式
            if encoder_codec_id == ffmpeg::codec::Id::AAC {
                let decoder_format = decoder.format();
                log::info!("原始音频格式: {:?}", decoder_format);

                // AAC编码器只支持fltp格式，直接设置
                let aac_format = ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar);
                encoder.set_format(aac_format);

                // 验证格式是否被接受
                if encoder.format() == aac_format {
                    log::info!("成功设置AAC编码器格式: {:?} (fltp)", aac_format);
                } else {
                    log::error!(
                        "AAC编码器格式设置失败，期望: {:?}, 实际: {:?}",
                        aac_format,
                        encoder.format()
                    );
                    return Err("AAC编码器不支持所需的音频格式".to_string());
                }
            } else {
                // 对于其他编码器，使用更通用的格式
                encoder.set_format(ffmpeg::format::Sample::I16(
                    ffmpeg::format::sample::Type::Packed,
                ));
            }

            encoder.set_bit_rate(128000); // 128kbps
            encoder.set_time_base((1, decoder.rate() as i32));

            // 尝试打开编码器
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
                    Self::process_video_packet(
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
                    Self::process_audio_packet(
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
    fn process_video_packet(
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

    /// 改进的音频包处理，支持AAC编码器的帧大小限制和格式转换
    fn process_audio_packet(
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
            let decoder_format = decoded_frame.format();
            let encoder_format = encoder.format();
            let decoder_rate = decoded_frame.rate();
            let encoder_rate = encoder.rate();
            let decoder_layout = decoded_frame.channel_layout();
            let encoder_layout = encoder.channel_layout();

            // 检查是否需要重采样
            let needs_resampling = decoder_format != encoder_format
                || decoder_rate != encoder_rate
                || decoder_layout != encoder_layout;

            let processed_frame = if needs_resampling {
                log::info!(
                    "音频参数不匹配，启用重采样 - 格式: {:?}->{:?}, 采样率: {}->{}, 声道: {:?}->{:?}",
                    decoder_format, encoder_format, decoder_rate, encoder_rate, decoder_layout, encoder_layout
                );

                // 使用重采样器进行格式转换
                match Self::resample_audio_frame(
                    &decoded_frame,
                    encoder_format,
                    encoder_rate,
                    encoder_layout,
                ) {
                    Ok(resampled_frame) => {
                        // 检查重采样后的帧是否有效
                        if resampled_frame.samples() == 0 {
                            log::debug!("重采样产生空帧，跳过此帧");
                            continue;
                        }
                        resampled_frame
                    }
                    Err(e) => {
                        log::warn!("音频重采样失败: {}, 跳过此帧", e);
                        continue;
                    }
                }
            } else {
                // 无需重采样，克隆原始帧以避免移动问题
                decoded_frame.clone()
            };

            // 处理音频帧大小限制，特别是针对AAC编码器
            let input_samples = processed_frame.samples();
            let max_encoder_samples = 1024; // AAC编码器的标准帧大小限制

            log::debug!(
                "输入音频帧: {} 采样点, 编码器最大支持: {} 采样点",
                input_samples,
                max_encoder_samples
            );

            if input_samples <= max_encoder_samples {
                // 帧大小在限制范围内，直接编码
                Self::encode_audio_frame(
                    &processed_frame,
                    encoder,
                    output_context,
                    output_stream_index,
                    frame_count,
                )?;
            } else {
                // 帧过大，需要分割处理
                log::debug!("音频帧过大，需要分割成多个子帧");
                Self::split_and_encode_audio_frame(
                    &processed_frame,
                    encoder,
                    output_context,
                    output_stream_index,
                    frame_count,
                    max_encoder_samples,
                )?;
            }
        }

        Ok(())
    }

    /// 使用重采样器进行音频格式转换
    fn resample_audio_frame(
        input_frame: &ffmpeg::frame::Audio,
        target_format: ffmpeg::format::Sample,
        target_rate: u32,
        target_layout: ffmpeg::channel_layout::ChannelLayout,
    ) -> Result<ffmpeg::frame::Audio, String> {
        // 检查是否真的需要重采样
        if input_frame.format() == target_format
            && input_frame.rate() == target_rate
            && input_frame.channel_layout() == target_layout
        {
            log::debug!("音频参数完全匹配，直接返回原始帧");
            return Ok(input_frame.clone());
        }

        // 创建重采样器上下文
        let mut resampler = ffmpeg::software::resampling::Context::get(
            input_frame.format(),
            input_frame.channel_layout(),
            input_frame.rate(),
            target_format,
            target_layout,
            target_rate,
        )
        .map_err(|e| format!("创建重采样器失败: {}", e))?;

        // 估算输出采样数（稍微多估算一些以确保足够的空间）
        let input_samples = input_frame.samples();
        let estimated_output_samples = if input_frame.rate() != target_rate {
            ((input_samples as u64 * target_rate as u64) / input_frame.rate() as u64) as usize
                + 2048 // 增加更多缓冲空间
        } else {
            input_samples + 2048
        };

        // 创建输出帧
        let mut output_frame =
            ffmpeg::frame::Audio::new(target_format, estimated_output_samples, target_layout);
        output_frame.set_rate(target_rate);

        // 复制时间戳
        if let Some(pts) = input_frame.pts() {
            // 调整时间戳以适应新的采样率
            let adjusted_pts = if input_frame.rate() != target_rate {
                (pts * target_rate as i64) / input_frame.rate() as i64
            } else {
                pts
            };
            output_frame.set_pts(Some(adjusted_pts));
        }

        // 执行重采样
        let result = resampler.run(input_frame, &mut output_frame);

        match result {
            Ok(Some(delay)) => {
                log::debug!("重采样完成，延迟信息: {:?}", delay);
                Self::handle_resampler_output(
                    output_frame,
                    input_samples,
                    estimated_output_samples,
                    target_format,
                    target_rate,
                    target_layout,
                    &mut resampler,
                )
            }
            Ok(None) => {
                log::debug!("重采样完成但无延迟信息");
                Self::handle_resampler_output(
                    output_frame,
                    input_samples,
                    estimated_output_samples,
                    target_format,
                    target_rate,
                    target_layout,
                    &mut resampler,
                )
            }
            Err(e) => Err(format!("重采样执行失败: {}", e)),
        }
    }

    /// 处理重采样器输出的通用逻辑
    fn handle_resampler_output(
        mut output_frame: ffmpeg::frame::Audio,
        input_samples: usize,
        estimated_output_samples: usize,
        target_format: ffmpeg::format::Sample,
        target_rate: u32,
        target_layout: ffmpeg::channel_layout::ChannelLayout,
        resampler: &mut ffmpeg::software::resampling::Context,
    ) -> Result<ffmpeg::frame::Audio, String> {
        let mut actual_samples = output_frame.samples();

        // 如果没有输出，尝试刷新缓冲区
        if actual_samples == 0 {
            log::debug!("重采样器暂无输出，尝试刷新缓冲区");
            let flush_result = resampler.flush(&mut output_frame);
            match flush_result {
                Ok(Some(_)) => {
                    actual_samples = output_frame.samples();
                    if actual_samples == 0 {
                        log::debug!("刷新后仍无输出，可能需要更多输入数据");
                        // 对于某些重采样器，可能需要多次输入才能产生输出
                        // 返回一个空帧，但不算作错误
                        let empty_frame =
                            ffmpeg::frame::Audio::new(target_format, 0, target_layout);
                        return Ok(empty_frame);
                    }
                    log::debug!("刷新缓冲区后获得 {} 采样点", actual_samples);
                }
                Ok(None) => {
                    log::debug!("刷新缓冲区无输出，返回空帧");
                    let empty_frame = ffmpeg::frame::Audio::new(target_format, 0, target_layout);
                    return Ok(empty_frame);
                }
                Err(e) => {
                    log::warn!("刷新重采样器缓冲区失败: {}", e);
                    let empty_frame = ffmpeg::frame::Audio::new(target_format, 0, target_layout);
                    return Ok(empty_frame);
                }
            }
        }

        // 如果有输出，处理帧大小调整
        if actual_samples > 0 {
            if actual_samples != estimated_output_samples {
                // 创建正确大小的帧
                let mut final_frame =
                    ffmpeg::frame::Audio::new(target_format, actual_samples, target_layout);
                final_frame.set_rate(target_rate);
                if let Some(pts) = output_frame.pts() {
                    final_frame.set_pts(Some(pts));
                }

                // 复制音频数据
                let source_data = output_frame.data(0);
                let dest_data = final_frame.data_mut(0);
                let channels = target_layout.channels() as usize;
                let bytes_per_sample = match target_format {
                    ffmpeg::format::Sample::I16(_) => 2,
                    ffmpeg::format::Sample::I32(_) => 4,
                    ffmpeg::format::Sample::F32(_) => 4,
                    ffmpeg::format::Sample::F64(_) => 8,
                    _ => 4, // 默认4字节
                };
                let bytes_to_copy = actual_samples * channels * bytes_per_sample;
                let copy_size = std::cmp::min(
                    bytes_to_copy,
                    std::cmp::min(source_data.len(), dest_data.len()),
                );

                if copy_size > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            source_data.as_ptr(),
                            dest_data.as_mut_ptr(),
                            copy_size,
                        );
                    }
                }

                log::debug!("重采样完成: {} -> {} 采样点", input_samples, actual_samples);
                Ok(final_frame)
            } else {
                log::debug!("重采样完成: {} -> {} 采样点", input_samples, actual_samples);
                Ok(output_frame)
            }
        } else {
            // 如果最终还是没有输出，返回空帧
            log::debug!("重采样最终无输出，返回空帧");
            let empty_frame = ffmpeg::frame::Audio::new(target_format, 0, target_layout);
            Ok(empty_frame)
        }
    }

    /// 编码单个音频帧
    fn encode_audio_frame(
        frame: &ffmpeg::frame::Audio,
        encoder: &mut ffmpeg::encoder::Audio,
        output_context: &mut ffmpeg::format::context::Output,
        output_stream_index: usize,
        frame_count: &mut i64,
    ) -> Result<(), String> {
        let frame_samples = frame.samples();

        // 检查帧大小是否符合编码器要求
        if frame_samples != 1024 {
            log::warn!(
                "音频帧大小不符合AAC编码器要求: {} samples，期望 1024 samples，跳过此帧",
                frame_samples
            );
            return Ok(());
        }

        let mut audio_frame = frame.clone();
        audio_frame.set_pts(Some(*frame_count));
        *frame_count += audio_frame.samples() as i64;

        match encoder.send_frame(&audio_frame) {
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
                log::debug!("成功编码音频帧: {} 采样点", frame_samples);
                Ok(())
            }
            Err(e) => {
                log::warn!("发送音频帧到编码器失败: {}, 跳过此帧", e);
                Ok(())
            }
        }
    }

    /// 分割大音频帧并编码
    fn split_and_encode_audio_frame(
        frame: &ffmpeg::frame::Audio,
        encoder: &mut ffmpeg::encoder::Audio,
        output_context: &mut ffmpeg::format::context::Output,
        output_stream_index: usize,
        frame_count: &mut i64,
        max_samples: usize,
    ) -> Result<(), String> {
        let total_samples = frame.samples();
        let channels = frame.channels();
        let format = frame.format();
        let rate = frame.rate();
        let channel_layout = frame.channel_layout();

        // 计算需要分割成多少个子帧
        let num_subframes = (total_samples + max_samples - 1) / max_samples;
        log::debug!(
            "将 {} 采样点的帧分割成 {} 个子帧，音频格式: {:?}, 声道数: {}",
            total_samples,
            num_subframes,
            format,
            channels
        );

        // 检查音频格式类型
        let is_planar = match format {
            ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Planar) => true,
            ffmpeg::format::Sample::I32(ffmpeg::format::sample::Type::Planar) => true,
            ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar) => true,
            ffmpeg::format::Sample::F64(ffmpeg::format::sample::Type::Planar) => true,
            _ => false,
        };

        // 计算每个采样点的字节数
        let bytes_per_sample = match format {
            ffmpeg::format::Sample::I16(_) => 2,
            ffmpeg::format::Sample::I32(_) => 4,
            ffmpeg::format::Sample::F32(_) => 4,
            ffmpeg::format::Sample::F64(_) => 8,
            _ => {
                log::warn!("不支持的音频格式: {:?}, 跳过分割", format);
                return Ok(());
            }
        };

        // 验证原始帧的数据完整性并检测实际声道配置
        let actual_channels = if is_planar {
            // 对于平面格式，检查每个声道的数据并检测实际可用声道数
            let mut detected_channels = 0u16;
            let mut has_valid_data = false;

            for ch in 0..channels {
                let channel_data = frame.data(ch as usize);
                let expected_bytes = total_samples * bytes_per_sample;

                if !channel_data.is_empty() && channel_data.len() >= expected_bytes {
                    detected_channels = ch + 1;
                    has_valid_data = true;
                    log::debug!(
                        "声道 {} 数据大小: {} bytes (期望: {} bytes) ✓",
                        ch,
                        channel_data.len(),
                        expected_bytes
                    );
                } else if !channel_data.is_empty() {
                    log::warn!(
                        "声道 {} 数据不足: {} bytes < {} bytes (期望)",
                        ch,
                        channel_data.len(),
                        expected_bytes
                    );
                } else {
                    log::debug!("声道 {} 数据为空", ch);
                    // 如果遇到空的声道，停止检查后续声道
                    break;
                }
            }

            if !has_valid_data {
                log::error!("所有声道都没有数据，无法进行音频分割");
                return Ok(());
            }

            if detected_channels != channels {
                log::info!(
                    "检测到实际声道数 {} 与声道布局声道数 {} 不匹配",
                    detected_channels,
                    channels
                );
            }

            detected_channels
        } else {
            // 对于交错格式，所有声道数据在data(0)中
            let frame_data = frame.data(0);
            let expected_bytes = total_samples * bytes_per_sample * channels as usize;

            if frame_data.is_empty() {
                log::error!("交错音频数据为空，无法进行音频分割");
                return Ok(());
            }

            if frame_data.len() < expected_bytes {
                log::warn!(
                    "交错音频数据不足: {} bytes < {} bytes (期望)",
                    frame_data.len(),
                    expected_bytes
                );
            } else {
                log::debug!(
                    "交错音频数据大小: {} bytes (期望: {} bytes) ✓",
                    frame_data.len(),
                    expected_bytes
                );
            }

            // 对于交错格式，假设声道数是正确的
            channels
        };

        // 根据实际声道数调整声道布局
        let actual_channel_layout = if actual_channels == 1 {
            ffmpeg::channel_layout::ChannelLayout::MONO
        } else if actual_channels == 2 {
            ffmpeg::channel_layout::ChannelLayout::STEREO
        } else {
            // 保持原有声道布局
            channel_layout
        };

        log::debug!(
            "使用实际声道配置: {} 声道, 布局: {:?}",
            actual_channels,
            actual_channel_layout
        );
        for i in 0..num_subframes {
            let start_sample = i * max_samples;
            let end_sample = std::cmp::min(start_sample + max_samples, total_samples);
            let subframe_samples = end_sample - start_sample;

            if subframe_samples == 0 {
                break;
            }

            // 创建新的音频帧，使用实际的声道配置
            let mut subframe =
                ffmpeg::frame::Audio::new(format, subframe_samples, actual_channel_layout);
            subframe.set_rate(rate);
            subframe.set_pts(Some(*frame_count));

            log::debug!(
                "处理子帧 {}/{}: 采样点范围 {}-{} ({} 采样点), 使用 {} 声道",
                i + 1,
                num_subframes,
                start_sample,
                end_sample,
                subframe_samples,
                actual_channels
            );

            // 复制音频数据
            let copy_success = if is_planar {
                // 平面格式：每个声道分别复制
                Self::copy_planar_audio_data(
                    frame,
                    &mut subframe,
                    start_sample,
                    subframe_samples,
                    actual_channels, // 使用实际声道数
                    bytes_per_sample,
                )
            } else {
                // 交错格式：所有声道数据在一起
                Self::copy_interleaved_audio_data(
                    frame,
                    &mut subframe,
                    start_sample,
                    subframe_samples,
                    actual_channels, // 使用实际声道数
                    bytes_per_sample,
                )
            };

            if copy_success {
                log::debug!(
                    "编码子帧 {}/{}: {} 采样点",
                    i + 1,
                    num_subframes,
                    subframe_samples
                );

                // 编码这个子帧
                Self::encode_audio_frame(
                    &subframe,
                    encoder,
                    output_context,
                    output_stream_index,
                    frame_count,
                )?;
            } else {
                log::warn!("音频数据复制失败，跳过子帧 {}", i + 1);
            }
        }

        Ok(())
    }

    /// 复制平面音频数据
    fn copy_planar_audio_data(
        src_frame: &ffmpeg::frame::Audio,
        dst_frame: &mut ffmpeg::frame::Audio,
        start_sample: usize,
        samples: usize,
        channels: u16,
        bytes_per_sample: usize,
    ) -> bool {
        // 复制指定数量的声道数据
        for ch in 0..channels {
            let src_data = src_frame.data(ch as usize);
            let dst_data = dst_frame.data_mut(ch as usize);

            // 检查源数据是否存在
            if src_data.is_empty() {
                log::warn!("声道 {} 源数据为空，跳过", ch);
                continue;
            }

            let start_byte = start_sample * bytes_per_sample;
            let length_bytes = samples * bytes_per_sample;

            // 检查源数据边界
            if start_byte + length_bytes > src_data.len() {
                log::warn!(
                    "声道 {} 数据越界: 需要 {} bytes (起始: {}, 长度: {})，但只有 {} bytes",
                    ch,
                    start_byte + length_bytes,
                    start_byte,
                    length_bytes,
                    src_data.len()
                );
                return false;
            }

            // 检查目标缓冲区大小
            if length_bytes > dst_data.len() {
                log::warn!(
                    "声道 {} 目标缓冲区不足: 需要 {} bytes，但只有 {} bytes",
                    ch,
                    length_bytes,
                    dst_data.len()
                );
                return false;
            }

            // 安全复制数据
            unsafe {
                let src_ptr = src_data.as_ptr().add(start_byte);
                let dst_ptr = dst_data.as_mut_ptr();
                std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, length_bytes);
            }

            log::debug!(
                "声道 {} 复制完成: {} bytes (采样点 {}-{})",
                ch,
                length_bytes,
                start_sample,
                start_sample + samples
            );
        }

        true
    }

    /// 复制交错音频数据
    fn copy_interleaved_audio_data(
        src_frame: &ffmpeg::frame::Audio,
        dst_frame: &mut ffmpeg::frame::Audio,
        start_sample: usize,
        samples: usize,
        channels: u16,
        bytes_per_sample: usize,
    ) -> bool {
        let src_data = src_frame.data(0);
        let dst_data = dst_frame.data_mut(0);

        if src_data.is_empty() {
            log::warn!("交错音频源数据为空");
            return false;
        }

        let bytes_per_frame = bytes_per_sample * channels as usize;
        let start_byte = start_sample * bytes_per_frame;
        let length_bytes = samples * bytes_per_frame;

        log::debug!(
            "交错音频复制参数: 起始采样点={}, 采样点数={}, 声道数={}, 每采样点字节数={}, 每帧字节数={}",
            start_sample,
            samples,
            channels,
            bytes_per_sample,
            bytes_per_frame
        );

        // 检查源数据边界
        if start_byte + length_bytes > src_data.len() {
            log::warn!(
                "交错音频数据越界: 需要 {} bytes (起始: {}, 长度: {})，但只有 {} bytes",
                start_byte + length_bytes,
                start_byte,
                length_bytes,
                src_data.len()
            );
            return false;
        }

        // 检查目标缓冲区大小
        if length_bytes > dst_data.len() {
            log::warn!(
                "交错音频目标缓冲区不足: 需要 {} bytes，但只有 {} bytes",
                length_bytes,
                dst_data.len()
            );
            return false;
        }

        // 安全复制数据
        unsafe {
            let src_ptr = src_data.as_ptr().add(start_byte);
            let dst_ptr = dst_data.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, length_bytes);
        }

        log::debug!(
            "交错音频复制完成: {} bytes (采样点 {}-{}, 覆盖 {} 个声道)",
            length_bytes,
            start_sample,
            start_sample + samples,
            channels
        );
        true
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
}
