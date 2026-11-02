#!/usr/bin/env bash
# 构建随包分发的精简版 ffmpeg / ffprobe。
#
# 关键取舍：
#   --disable-gpl             不含 libx264 / libx265，分发包不引入 GPL 组件
#   --enable-mediafoundation  保留 h264_mf（系统 Media Foundation 编码器）作为软编兜底
#   白名单裁剪               仅保留 QuickLook 预览转码实际用到的编解码器与封装
#
# 前置条件（MSYS2 MINGW64）：
#   pacman -S --needed base-devel mingw-w64-x86_64-gcc mingw-w64-x86_64-nasm make
#
# 用法：
#   ./scripts/build-ffmpeg.sh                  # x64
#   TARGET_ARCH=arm64 ./scripts/build-ffmpeg.sh
#
# 产物：src-tauri/binaries/{ffmpeg,ffprobe}-<triple>.exe

set -euo pipefail

FFMPEG_VERSION="${FFMPEG_VERSION:-8.1.1}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$ROOT_DIR/.ffmpeg-build}"
SRC_DIR="$BUILD_DIR/ffmpeg-$FFMPEG_VERSION"
ARCHIVE="$BUILD_DIR/ffmpeg-$FFMPEG_VERSION.tar.xz"
# ffprobe 用独立源码树构建：FFmpeg 不允许在已配置过的树里做第二种配置，
# 而两者的组件白名单差异极大，共用一个树会互相牵连。
FFPROBE_SRC_DIR="$BUILD_DIR/ffprobe-src/ffmpeg-$FFMPEG_VERSION"
OUT_DIR="$ROOT_DIR/src-tauri/binaries"

TARGET_ARCH="${TARGET_ARCH:-x64}"
case "$TARGET_ARCH" in
  x64)
    TRIPLE="x86_64-pc-windows-msvc"
    ARCH_FLAG="--arch=x86_64"
    ;;
  arm64)
    TRIPLE="aarch64-pc-windows-msvc"
    ARCH_FLAG="--arch=aarch64"
    ;;
  *)
    echo "不支持的 TARGET_ARCH: $TARGET_ARCH（可选 x64 / arm64）" >&2
    exit 1
    ;;
esac

log() { printf '\033[36m[build-ffmpeg]\033[0m %s\n' "$*"; }

# ── 平台与工具链检查 ───────────────────────────────────────────
case "$(uname -s)" in
  MINGW* | MSYS*) ;;
  *)
    echo "此脚本仅支持 Windows（MSYS2 / Git Bash 环境）" >&2
    exit 1
    ;;
esac

for cmd in make gcc curl tar; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "缺少命令: $cmd" >&2
    echo "请在 MSYS2 MINGW64 中安装: base-devel mingw-w64-x86_64-gcc mingw-w64-x86_64-nasm" >&2
    exit 1
  fi
done

mkdir -p "$BUILD_DIR" "$OUT_DIR"

# Git Bash / MSYS2 下 TMPDIR 若继承自 Windows 的 TEMP（形如 C:\Users\...\Temp），
# 反斜杠会被 MSYS 的路径转换吞掉，configure 的临时文件测试会拿到
# `C:Users...Temp/ffconf.xxx` 这种非法路径而失败。强制使用 MSYS 风格的 /tmp。
export TMPDIR=/tmp
mkdir -p "$TMPDIR"

# ── 获取源码 ───────────────────────────────────────────────────
if [[ ! -f "$ARCHIVE" ]]; then
  log "下载 ffmpeg $FFMPEG_VERSION 源码"
  curl -fL --retry 3 --progress-bar \
    -o "$ARCHIVE" "https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz"
fi

if [[ ! -d "$SRC_DIR" ]]; then
  log "解压源码到 $BUILD_DIR"
  tar -xf "$ARCHIVE" -C "$BUILD_DIR"
fi

cd "$SRC_DIR"

# ── 配置 ───────────────────────────────────────────────────────
# ARCH_FLAG 需要按空格拆分，故意不加引号
#
# --enable-d3d11va 是必需的：h264_mf 的实现（libavcodec/mfenc.c）无条件下引用
# ID3D11DeviceContext / AVD3D11VADeviceContext 等类型，d3d11va 关掉会直接编译失败。
# 而 --disable-autodetect 会把 d3d11va 一并关闭，所以必须显式打开。
#
# 滤镜白名单不能只看代码里有没有显式写 -vf，以下几项由 ffmpeg 自动插入：
#   transpose / hflip / vflip / rotate —— -autorotate 默认开启（ffmpeg_demux.c），
#     按 displaymatrix 的旋转角自动插入：90°/270° 用 transpose，180° 用 hflip+vflip，
#     非 90 倍数用 rotate。手机竖拍视频必然命中，删掉会让这类文件转码直接失败。
#   crop —— MP4 的 clap（Clean Aperture）box、matroska 的 crop 元数据会生成
#     AV_PKT_DATA_FRAME_CROPPING side data，apply_cropping 默认 CROP_ALL 而非 CROP_CODEC，
#     因此只要文件带该元数据就会插入 crop。
# 以上任一缺失，ffmpeg_filter.c 的 insert_filter() 返回 AVERROR_BUG，
# 报 "Error configuring filter graph" 后整条转码失败，不是降级也不是画质损失。
# wrapped_avframe 是 --disable-everything 下最容易漏的一项：
# probe_encoder() 用 `-f lavfi -i color=...` 造一帧再喂给编码器，lavfi 输出的
# 原始帧以 wrapped_avframe 包装，进入 -c:v 之前需要同名"解码器"接住。
# 缺它时报 "no decoder found for: wrapped_avframe"，试编必然失败 ——
# 而 select_encoder() 会把试编失败解读为"该编码器不可用"，最终 nvenc/amf/qsv/mf
# 全部落选，转码整体失败，且错误信息指向编码器而不是缺解码器，极难排查。
log "配置构建参数（arch=$TARGET_ARCH）"
# --disable-everything 必须排在所有 --enable-* 之前。它的实现是
# `unset ${XXX_LIST}`，即把各类别组件列表清空；之后的 --enable-* 往空列表里追加。
# 顺序颠倒会让白名单整体失效（等价于全量构建）。
./configure \
  $ARCH_FLAG \
  --target-os=mingw32 \
  --disable-everything \
  --disable-gpl \
  --disable-doc \
  --disable-debug \
  --disable-ffplay \
  --disable-ffprobe \
  --disable-network \
  --disable-autodetect \
  --enable-small \
  --enable-static \
  --disable-shared \
  --enable-mediafoundation \
  --enable-d3d11va \
  --enable-hwaccel=h264_d3d11va,hevc_d3d11va \
  --enable-decoder=h264,hevc,vp8,vp9,av1,mpeg4,mpeg2video,mpeg1video \
  --enable-decoder=msmpeg4v3,msmpeg4v2,wmv3,wmv2,vc1,prores,dnxhd,flv,theora,mjpeg,rawvideo,h263,vp6,vp6f \
  --enable-decoder=aac,mp3,mp2,ac3,eac3,dts,flac,opus,vorbis,alac,wmav2,truehd,amrnb,amrwb \
  --enable-decoder=pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le,pcm_u8,pcm_alaw,pcm_mulaw \
  --enable-decoder=wrapped_avframe \
  --enable-encoder=h264_mf,aac,wrapped_avframe \
  --enable-muxer=hls,mpegts,mp4,matroska,image2,null \
  --enable-demuxer=mov,matroska,mpegts,mpegps,avi,flv,asf,ogg,wav,mp3,aac,flac,h264,hevc,image2,rm \
  --enable-bsf=h264_mp4toannexb,hevc_mp4toannexb \
  --enable-protocol=file,pipe \
  --enable-filter=buffer,buffersink,abuffer,abuffersink,scale,format,aformat,aresample,transpose,hflip,vflip,rotate,crop,color,null,anull,fps,trim \
  --enable-indev=lavfi \
  --enable-parser=h264,hevc,vp9,av1,mpeg4video,mpegvideo,aac,ac3,dts,flac,opus,vorbis \
  --extra-cflags="-Os -ffunction-sections -fdata-sections" \
  --extra-ldflags="-static -Wl,--gc-sections"

# CONFIG_GPL 必须为 0，否则说明意外引入了 GPL 组件
if grep -qE '^#define CONFIG_GPL 1' config.h; then
  echo "构建配置仍包含 GPL 组件，请检查 configure 参数" >&2
  exit 1
fi

# h264_mf 是去掉 libx264 后唯一的软件编码回退，必须确认已启用。
# 注意：FFmpeg 6.1 起组件开关被拆到 config_components.h，不在 config.h 里。
if ! grep -qE '^#define CONFIG_H264_MF_ENCODER 1' config_components.h; then
  echo "h264_mf 编码器未启用，Media Foundation 支持不可用" >&2
  exit 1
fi

# ── 编译 ───────────────────────────────────────────────────────
JOBS="$(nproc 2>/dev/null || echo 4)"
log "编译中（-j$JOBS），首次构建约需 10-20 分钟"

# 用 -k 跑到尽可能多的目标。Windows 上 make 归档大库时会触发命令行长度上限
# （约 8191 字符），表现为 `ar` 拿到空的 .objs 响应文件，最终产出 8 字节的空归档。
# 这类失败不影响已编译的 .o，下面统一修复。
make -k -j"$JOBS" || true

# ── 修复被截断的静态库 ─────────────────────────────────────────
# 判断依据：AR 归档文件至少含 "!<arch>\n" 8 字节头 + 符号表，
# 小于 1KB 视为空归档，改用 .o 列表文件重新归档（ar 的 @file 不受命令行长度限制）。
# 返回值 1 表示确实重建过，调用方据此决定是否需要重新链接。
fix_truncated_archives() {
  local tree="$1" rebuilt=0 archive size libdir rel

  for archive in "$tree"/*/lib*.a; do
    [[ -f "$archive" ]] || continue
    size=$(wc -c < "$archive")
    if [[ $size -ge 1024 ]]; then
      continue
    fi

    libdir=$(basename "$(dirname "$archive")")
    rel="$libdir/$(basename "$archive")"
    log "重建被截断的归档: $rel ($size 字节)"

    # ar / ranlib 是原生 Windows 程序，只在命令行参数上做 MSYS 路径转换，
    # 响应文件**内部**的路径不会被转换：写成 /e/... 会让 ar 报 No such file，
    # 而且归档会留成 8 字节的空壳。因此在树内用相对路径。
    (
      cd "$tree"
      find "$libdir" -name "*.o" | sort > "$libdir.objs"
      rm -f "$rel"
      ar rcD "$rel" "@$libdir.objs" || {
        echo "归档失败: $rel" >&2
        exit 1
      }
      ranlib "$rel"
    )

    size=$(wc -c < "$archive")
    if [[ $size -lt 1024 ]]; then
      echo "归档仍为空: $archive ($size 字节)" >&2
      exit 1
    fi
    rebuilt=1
  done

  return $rebuilt
}

# 归档修复后需要重新链接可执行文件（返回值 1 表示重建过）
if ! fix_truncated_archives "$SRC_DIR"; then
  log "静态库已重建，重新链接 ffmpeg"
fi
make -j"$JOBS"

# ── 安装并裁剪符号 ─────────────────────────────────────────────
install_bin() {
  local src_bin="$1" name="$2"

  if [[ ! -f "$src_bin" ]]; then
    echo "未生成 $src_bin，请检查 configure 参数" >&2
    exit 1
  fi

  local dst_bin="$OUT_DIR/$name-$TRIPLE.exe"
  cp "$src_bin" "$dst_bin"
  if command -v strip >/dev/null 2>&1; then
    strip "$dst_bin" || log "strip 失败，保留原始符号"
  fi
  log "已生成: $dst_bin ($(du -h "$dst_bin" | cut -f1))"
}

install_bin "$SRC_DIR/ffmpeg.exe" ffmpeg

# ── 阶段二：极简 ffprobe ───────────────────────────────────────
# ffprobe 只用来读取容器元数据（codec_type / codec_name / width / height /
# 音频的 sample_rate、channels），不输出任何帧，因此不需要滤镜、缩放、重采样，
# 也不需要编码器。用独立的源码树做一次 --disable-everything 的构建：18 MB → 4.25 MB。
#
# 唯一保留的解码器是 aac，原因是 FFmpeg 8.x 的 aac parser 只负责切帧：
# libavcodec/aac_ac3_parser.c 的 AAC 分支只写 profile / bit_rate，不把 ADTS 头里的
# 采样率和声道数写回 avctx（源码注释说明这两个字段在 HE-AAC 下不可靠，因为 SBR/PS
# 会让实际输出翻倍或改变声道数）。于是 ts / m2ts / 裸 ADTS 里封装的 AAC，
# avformat_find_stream_info 只能靠解码器纠正参数；没有解码器时 sample_rate 和
# channels 恒为 0（mp4/mkv 走容器的 AudioSpecificConfig，不受影响）。
# 代价实测：2.59 MiB → 4.25 MiB（+1.66 MiB，aacdec 及其 DCT/FFT 依赖），
# 换来与完整版 ffprobe 一致的音频参数，也避免将来读音频信息时踩这个坑。
#
# 之所以另开一棵树而不是复用上面那棵：FFmpeg 不允许在已配置过的源码目录里
# 换一套参数重新配置，而两者的白名单差异极大，互相牵连只会让维护变难。
#
# 只调主构建时设 SKIP_FFPROBE=1，可跳过这段无谓的重编。
if [[ "${SKIP_FFPROBE:-0}" = "1" ]]; then
  log "已跳过 ffprobe 构建（SKIP_FFPROBE=1）"
  exit 0
fi

if [[ ! -d "$FFPROBE_SRC_DIR" ]]; then
  log "解压第二份源码用于 ffprobe"
  mkdir -p "$(dirname "$FFPROBE_SRC_DIR")"
  tar -xf "$ARCHIVE" -C "$(dirname "$FFPROBE_SRC_DIR")"
fi

cd "$FFPROBE_SRC_DIR"

log "配置极简 ffprobe（arch=$TARGET_ARCH）"
./configure \
  $ARCH_FLAG \
  --target-os=mingw32 \
  --disable-everything \
  --disable-gpl \
  --disable-doc \
  --disable-debug \
  --disable-network \
  --disable-autodetect \
  --disable-avdevice \
  --disable-avfilter \
  --disable-swscale \
  --disable-swresample \
  --disable-ffmpeg \
  --disable-ffplay \
  --enable-ffprobe \
  --enable-static \
  --disable-shared \
  --enable-small \
  --enable-protocol=file,pipe \
  --enable-demuxer=mov,matroska,mpegts,mpegps,avi,flv,asf,ogg,wav,mp3,aac,flac,h264,hevc,image2,hls,rm \
  --enable-parser=h264,hevc,vp9,av1,mpeg4video,mpegvideo,aac,ac3,dts,flac,opus,vorbis,mpegaudio \
  --enable-decoder=aac \
  --extra-cflags="-Os -ffunction-sections -fdata-sections" \
  --extra-ldflags="-static -Wl,--gc-sections"

# aac 解码器是 ts 内 AAC 参数的唯一来源，缺了它 ffprobe 会静默输出 0，不会报错，
# 属于那种构建成功、运行也不报错、只在读音频参数时才暴露的缺陷，因此显式校验。
if ! grep -qE '^#define CONFIG_AAC_DECODER 1' config_components.h; then
  echo "ffprobe 缺少 aac 解码器，ts 内 AAC 的采样率/声道数将无法探测" >&2
  exit 1
fi

if grep -qE '^#define CONFIG_GPL 1' config.h; then
  echo "ffprobe 构建配置包含 GPL 组件，请检查 configure 参数" >&2
  exit 1
fi

log "编译 ffprobe（-j$JOBS）"
make -k -j"$JOBS" || true
if ! fix_truncated_archives "$FFPROBE_SRC_DIR"; then
  log "静态库已重建，重新链接 ffprobe"
fi
make -j"$JOBS"

install_bin "$FFPROBE_SRC_DIR/ffprobe.exe" ffprobe

log "完成。Tauri 打包时将按 target triple 自动选取对应架构的二进制。"
