#!/bin/sh
# Run from an empty build directory. CC must target static Linux musl.
set -eu
archive=${1:?Pass the path to the Jellyfin FFmpeg source archive}
arch=${2:?Pass x86_64 or aarch64}
zlib_archive=${3:-$(dirname "$archive")/zlib-1.3.2.tar.gz}
: "${CC:?Set CC to the musl compiler}"
# The MMXEXT vertical scaler needs external x86 assembly. Its inline-only path
# can produce incorrect pixels when built with --disable-x86asm.
case "$arch" in
  x86_64) set -- --disable-mmxext ;;
  aarch64) set -- ;;
  *) exit 2 ;;
esac
pin=7367430190d99cd756470e24d9cb5f64e1f60977d226c3cbcf41c79151e20ab9
printf '%s  %s\n' "$pin" "$archive" | sha256sum -c -
printf '%s  %s\n' bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16 "$zlib_archive" | sha256sum -c -
mkdir zlib-src
tar -xf "$zlib_archive" -C zlib-src --strip-components=1
(
  cd zlib-src
  CC="$CC" AR="${CROSS_PREFIX:-}ar" RANLIB="${CROSS_PREFIX:-}ranlib" ./configure --static
  make -j"${JOBS:-2}" libz.a
)
mkdir ffmpeg-src
tar -xf "$archive" -C ffmpeg-src --strip-components=1
cd ffmpeg-src
./configure --arch="$arch" --enable-cross-compile --cross-prefix="${CROSS_PREFIX:-}" --target-os=linux \
  --cc="$CC" --host-cc="${HOST_CC:-cc}" --extra-cflags=-I../zlib-src --extra-ldflags="-static -L../zlib-src" \
  --disable-autodetect --disable-shared --enable-static --disable-debug \
  --disable-doc --disable-ffplay --disable-x86asm "$@" --disable-devices \
  --disable-network --disable-v4l2-m2m --disable-everything --enable-ffmpeg --enable-ffprobe --enable-zlib \
  --enable-protocol=file,pipe,fd \
  --enable-demuxer=dts,mov,matroska,mpegts,mpegvideo,avi,aac,mp3,flac,wav,ogg,asf,ac3,hevc,h264,webvtt,srt,image2,image2pipe \
  --enable-parser=dca,aac,h264,hevc,mpegaudio,mpegvideo,flac,av1,vp9,mjpeg,png,webp \
  --enable-decoder=dca,h264,hevc,mpeg4,mpeg2video,av1,vp8,vp9,aac,flac,mp3,opus,vorbis,ac3,eac3,pcm_s16le,pcm_s24le,mjpeg,png,webp,rawvideo \
  --enable-encoder=aac,flac,pcm_s16le,mjpeg,mpeg4 \
  --enable-muxer=mp4,mov,matroska,mpegts,adts,flac,wav,pcm_s16le,null,image2,image2pipe \
  --enable-filter=apad,asetpts,atrim,loudnorm,ebur128,scale,format,aresample,aformat,testsrc2,color,sine,anull,null --enable-avfilter
make -j"${JOBS:-2}" ffmpeg ffprobe
cd ..
mkdir media-tools
cp ffmpeg-src/ffmpeg ffmpeg-src/ffprobe ffmpeg-src/COPYING.LGPLv2.1 media-tools/
cp "$zlib_archive" media-tools/zlib-1.3.2.tar.gz
