# syntax=docker/dockerfile:1
#
# 构建上下文是仓库根目录：
#   docker build -f packaging/docker/Dockerfile -t palveni-server .

# ---- 1. Web 工作台 ----
FROM node:24-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# ---- 2. 服务端 ----
# 版本与 rust-toolchain.toml 保持一致。
FROM rust:1.100-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
COPY packaging/smb-helper/ packaging/smb-helper/
COPY wit/ wit/
COPY plugins/ plugins/
# embed-web 在编译期把 web/dist 打进二进制。
COPY --from=web /src/web/dist web/dist
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo +1.100.0 build --release --locked -p palveni-server --features embed-web \
        --message-format=json-render-diagnostics > /tmp/palveni-build.jsonl \
 && bundle_dir="$(perl -MJSON::PP -ne '$m=decode_json($_); print "$m->{out_dir}/skin-packages" if $m->{reason} eq "build-script-executed" && $m->{package_id} =~ m{[/#]palveni-plugin-host[@#]};' /tmp/palveni-build.jsonl)" \
 && cp -r "$bundle_dir" /usr/local/bin/skin-packages \
 && cp target/release/palveni /usr/local/bin/palveni
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo +1.100.0 build --manifest-path packaging/smb-helper/Cargo.toml --target-dir target --release --locked \
 && cp target/release/palveni-smb-helper /usr/local/bin/palveni-smb-helper

# ---- 3. 运行镜像 ----
FROM debian:bookworm-slim AS runtime
ARG TARGETARCH
# 捆绑 jellyfin-ffmpeg。
# 升级时三个值一起改；哈希取自上游同目录的 .buildinfo 文件。
ARG FFMPEG_VERSION=8.1.3-2
ARG FFMPEG_SHA256_AMD64=75558a2282c6f6f3ce88dd10e87c4b4d3af4736a1a1c812a57c1c57f849b58f6
ARG FFMPEG_SHA256_ARM64=de728a7e4cb2a6458b043afbb1c6d59f2df35ab894070c010ba8d51db6349e12

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates curl tini; \
    case "${TARGETARCH}" in \
      amd64) ffmpeg_sha256="${FFMPEG_SHA256_AMD64}" ;; \
      arm64) ffmpeg_sha256="${FFMPEG_SHA256_ARM64}" ;; \
      *) echo "unsupported architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    curl -fsSL -o /tmp/ffmpeg.deb \
      "https://repo.jellyfin.org/files/ffmpeg/debian/8.x/${FFMPEG_VERSION}/${TARGETARCH}/jellyfin-ffmpeg8_${FFMPEG_VERSION}-bookworm_${TARGETARCH}.deb"; \
    echo "${ffmpeg_sha256}  /tmp/ffmpeg.deb" | sha256sum -c -; \
    apt-get install -y --no-install-recommends /tmp/ffmpeg.deb; \
    rm -rf /tmp/ffmpeg.deb /var/lib/apt/lists/*; \
    /usr/lib/jellyfin-ffmpeg/ffmpeg -version | head -n 1

COPY --from=build /usr/local/bin/palveni /usr/local/bin/palveni
COPY --from=build /usr/local/bin/palveni-smb-helper /usr/local/bin/palveni-smb-helper
COPY --from=build /usr/local/bin/skin-packages/ /usr/local/bin/skin-packages/

# 以非 root 运行。媒体目录的属主不是 1000 时，用 `--user <uid>:<gid>` 覆盖，
# 并保证 /data 对该用户可写。
RUN mkdir -p /data && chown 1000:1000 /data
USER 1000:1000

ENV PALVENI_DATA_DIR=/data
VOLUME /data
EXPOSE 7258

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:7258/api/v1/system/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/palveni"]
CMD ["serve"]
