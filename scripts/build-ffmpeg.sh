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
if [[ ! -d "$SRC_DIR" ]]; then
  ARCHIVE="$BUILD_DIR/ffmpeg-$FFMPEG_VERSION.tar.xz"
  if [[ ! -f "$ARCHIVE" ]]; then
    log "下载 ffmpeg $FFMPEG_VERSION 源码"
    curl -fL --retry 3 --progress-bar \
      -o "$ARCHIVE" "https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz"
  fi
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
log "配置构建参数（arch=$TARGET_ARCH）"
./configure \
  $ARCH_FLAG \
  --target-os=mingw32 \
  --disable-gpl \
  --disable-doc \
  --disable-debug \
  --disable-ffplay \
  --disable-network \
  --disable-autodetect \
  --enable-small \
  --enable-static \
  --disable-shared \
  --enable-mediafoundation \
  --enable-d3d11va \
  --enable-hwaccel=h264_d3d11va,hevc_d3d11va \
  --enable-decoder=h264,hevc,vp8,vp9,av1,mpeg4,mpeg2video,mpeg1video \
  --enable-decoder=msmpeg4v3,wmv3,wmv2,vc1,prores,dnxhd,flv,theora,mjpeg,rawvideo,h263 \
  --enable-decoder=aac,mp3,ac3,eac3,dts,flac,opus,vorbis,alac,wmav2,truehd \
  --enable-decoder=pcm_s16le,pcm_s24le,pcm_f32le \
  --enable-encoder=h264_mf,aac \
  --enable-muxer=hls,mpegts,mp4,matroska,image2,null \
  --enable-demuxer=mov,matroska,mpegts,mpegps,avi,flv,asf,ogg,wav,mp3,aac,h264,hevc,image2 \
  --enable-bsf=h264_mp4toannexb,hevc_mp4toannexb \
  --enable-protocol=file,pipe \
  --enable-filter=scale,format,aformat,aresample,transpose,crop,color,null,anull \
  --enable-indev=lavfi \
  --enable-parser=h264,hevc,vp9,av1,mpeg4video,mpegvideo,aac,ac3,dts,flac,opus,vorbis \
  --extra-cflags="-O2" \
  --extra-ldflags="-static"

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
make -j"$JOBS"

# ── 安装并裁剪符号 ─────────────────────────────────────────────
for bin in ffmpeg ffprobe; do
  src_bin="$SRC_DIR/$bin.exe"
  if [[ ! -f "$src_bin" ]]; then
    echo "未生成 $src_bin，请确认未启用 --disable-$bin" >&2
    exit 1
  fi
  dst_bin="$OUT_DIR/$bin-$TRIPLE.exe"
  cp "$src_bin" "$dst_bin"
  if command -v strip >/dev/null 2>&1; then
    strip "$dst_bin" || log "strip 失败，保留原始符号"
  fi
  log "已生成: $dst_bin ($(du -h "$dst_bin" | cut -f1))"
done

log "完成。Tauri 打包时将按 target triple 自动选取对应架构的二进制。"
