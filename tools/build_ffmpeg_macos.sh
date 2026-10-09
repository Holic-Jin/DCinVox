#!/bin/bash
# 배포용 ffmpeg (LGPL, GPL·nonfree 구성요소 없음) 을 소스에서 빌드.
# 필요한 형식만 넣어 작게 만들고, 하드웨어 인코더(VideoToolbox)로 H.264 인코딩.
# 사용: tools/build_ffmpeg_macos.sh  → src-tauri/binaries/ffmpeg-<target>
# LGPL 준수: 이 스크립트와 소스 tarball(third_party/)을 함께 보관·제공한다.
set -euo pipefail
VER=9.0.2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC_TAR="$ROOT/third_party/ffmpeg-$VER.tar.xz"
WORK="${TMPDIR:-/tmp}/dcinvox-ffmpeg-build"
ARCH="$(uname -m)"
case "$ARCH" in
  arm64) TARGET=aarch64-apple-darwin ;;
  x86_64) TARGET=x86_64-apple-darwin ;;
esac
[ -f "$SRC_TAR" ] || curl -fL --retry 3 -o "$SRC_TAR" "https://ffmpeg.org/releases/ffmpeg-$VER.tar.xz"
rm -rf "$WORK" && mkdir -p "$WORK"
tar -xf "$SRC_TAR" -C "$WORK"
cd "$WORK/ffmpeg-$VER"
./configure \
  --prefix="$WORK/out" \
  --disable-gpl --disable-nonfree --disable-autodetect \
  --enable-videotoolbox --enable-audiotoolbox \
  --enable-static --disable-shared \
  --disable-doc --disable-debug --disable-ffplay --disable-ffprobe --disable-network \
  --extra-cflags="-mmacosx-version-min=11.0" --extra-ldflags="-mmacosx-version-min=11.0" \
  --disable-everything \
  --enable-protocol=file,pipe \
  --enable-demuxer=mov,matroska,avi,flv,mpegts,mpegps,asf,wav,mp3,aac,flac,ogg,concat \
  --enable-decoder=h264,hevc,vp8,vp9,mpeg4,mpeg2video,mpeg1video,msmpeg4v3,wmv2,wmv3,vc1,flv,mjpeg,prores \
  --enable-decoder=aac,aac_latm,mp3,mp3float,mp2,opus,vorbis,flac,alac,ac3,eac3,wmav2,wmapro \
  --enable-decoder=pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le,pcm_s16be,pcm_u8,wrapped_avframe \
  --enable-encoder=h264_videotoolbox,aac,pcm_s16le \
  --enable-muxer=mp4,mov,wav,null \
  --enable-parser=h264,hevc,aac,mpegaudio,vp8,vp9,mpeg4video,ac3,flac,opus,vorbis \
  --enable-bsf=h264_mp4toannexb,aac_adtstoasc \
  --enable-filter=scale,fps,pad,setsar,format,setpts,aresample,aformat,anull,null,color,volumedetect,copy \
  --enable-indev=lavfi \
  --enable-swscale --enable-swresample
make -j"$(sysctl -n hw.ncpu)"
mkdir -p "$ROOT/src-tauri/binaries"
cp ffmpeg "$ROOT/src-tauri/binaries/ffmpeg-$TARGET"
strip "$ROOT/src-tauri/binaries/ffmpeg-$TARGET"
echo "built: $ROOT/src-tauri/binaries/ffmpeg-$TARGET"
"$ROOT/src-tauri/binaries/ffmpeg-$TARGET" -hide_banner -version | head -3
