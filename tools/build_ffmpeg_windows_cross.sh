#!/bin/bash
# Windows용 배포 ffmpeg.exe (LGPL) 를 Linux에서 mingw-w64로 교차 빌드 (GitHub Actions ubuntu에서 실행).
# H.264 인코딩은 Windows 내장 Media Foundation(h264_mf) 사용 — GPL인 x264를 쓰지 않음.
# 사용: sudo apt-get install -y mingw-w64 nasm && tools/build_ffmpeg_windows_cross.sh
set -euo pipefail
VER=9.0.2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC_TAR="$ROOT/third_party/ffmpeg-$VER.tar.xz"
WORK="${TMPDIR:-/tmp}/dcinvox-ffmpeg-win"
mkdir -p "$ROOT/third_party"
[ -f "$SRC_TAR" ] || curl -fL --retry 3 -o "$SRC_TAR" "https://ffmpeg.org/releases/ffmpeg-$VER.tar.xz"
rm -rf "$WORK" && mkdir -p "$WORK"
tar -xf "$SRC_TAR" -C "$WORK"
cd "$WORK/ffmpeg-$VER"
./configure \
  --target-os=mingw32 --arch=x86_64 --cross-prefix=x86_64-w64-mingw32- \
  --disable-gpl --disable-nonfree --disable-autodetect \
  --enable-mediafoundation --enable-d3d11va \
  --enable-static --disable-shared --extra-ldflags="-static" \
  --disable-doc --disable-debug --disable-ffplay --disable-ffprobe --disable-network \
  --disable-everything \
  --enable-protocol=file,pipe \
  --enable-demuxer=mov,matroska,avi,flv,mpegts,mpegps,asf,wav,mp3,aac,flac,ogg,concat \
  --enable-decoder=h264,hevc,vp8,vp9,mpeg4,mpeg2video,mpeg1video,msmpeg4v3,wmv2,wmv3,vc1,flv,mjpeg,prores \
  --enable-decoder=aac,aac_latm,mp3,mp3float,mp2,opus,vorbis,flac,alac,ac3,eac3,wmav2,wmapro \
  --enable-decoder=pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le,pcm_s16be,pcm_u8,wrapped_avframe \
  --enable-encoder=h264_mf,aac,pcm_s16le \
  --enable-muxer=mp4,mov,wav,null \
  --enable-parser=h264,hevc,aac,mpegaudio,vp8,vp9,mpeg4video,ac3,flac,opus,vorbis \
  --enable-bsf=h264_mp4toannexb,aac_adtstoasc \
  --enable-filter=scale,fps,pad,setsar,format,setpts,aresample,aformat,anull,null,color,volumedetect,copy \
  --enable-indev=lavfi \
  --enable-swscale --enable-swresample
make -j"$(nproc)"
mkdir -p "$ROOT/src-tauri/binaries"
cp ffmpeg.exe "$ROOT/src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe"
x86_64-w64-mingw32-strip "$ROOT/src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe"
echo "built: src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe"
