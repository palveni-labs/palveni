# Palveni Server

Palveni 是自托管媒体库服务。管理员可以用内置工作台添加本地媒体目录、扫描并浏览文件，查看任务进度和 FFmpeg 的硬件编码能力。默认使用 SQLite，也支持 PostgreSQL 和 MySQL。

## 用 Docker 启动

在仓库根目录构建镜像，将下面的 `/path/to/media` 换成宿主机上的媒体目录。

```sh
docker build -f packaging/docker/Dockerfile -t palveni-server .
docker run -d --name palveni \
  -p 127.0.0.1:7258:7258 \
  -v palveni-data:/data \
  -v /path/to/media:/media:ro \
  palveni-server
```

在本机浏览器打开 [工作台](http://localhost:7258)，创建管理员账号。在「系统 → 媒体库」添加 `/media`，开始扫描。数据卷 `palveni-data` 保存账号、媒体索引、设备密钥和已安装的皮肤。其他挂载方式及硬件加速配置见 [Compose 示例](packaging/docker/compose.yaml)。

## 从源码运行

工作台需要 Node.js 24。服务端使用 [仓库指定的 Rust 工具链](rust-toolchain.toml)。FFmpeg 和 FFprobe 作为外部程序调用，Docker 镜像已包含它们。

```sh
npm ci --prefix web
npm run build --prefix web
cargo run --locked -p palveni-server --features embed-web -- serve
```

`palveni doctor` 检查 FFmpeg、硬件编码能力和数据库连接。`palveni openapi` 将 OpenAPI 文档写到标准输出，运行中的服务也提供 `/api/v1/openapi.json`。

## 皮肤

在工作台的「应用 → 外观」安装官方皮肤，或上传第三方皮肤的 `.tar` 包。服务端会校验包内资源，切换皮肤无需重启。官方皮肤和社区投稿见 [Palveni 插件仓库](https://github.com/palveni-labs/plugins)。

## 许可证

服务端和工作台使用 [MPL-2.0](LICENSE)，插件接口使用 [Apache-2.0](wit/LICENSE)。第三方字体的许可证随各自资源分发。
