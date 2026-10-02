# imyemail-cloud

[简体中文 README](README.md) · [中文文档导航](docs/README.md) · [最新版本下载](https://github.com/logdns/imyemail-cloud/releases/latest)

imyemail-cloud 是一个 MIT 开源的多端原生邮件客户端项目，公开仓库为 [github.com/logdns/imyemail-cloud](https://github.com/logdns/imyemail-cloud)，产品链接统一为 [https://imy.email](https://imy.email)。当前版本为 `0.2.1`。

本仓库只包含客户端、共享邮件核心、可选推送桥、测试工具、构建脚本和开源文档。它不包含官网/Web 应用、支付、订阅、软件激活、设备名额或商业授权门禁；同步和发送等客户端功能不需要购买软件许可证。

## 下载

[下载最新版本](https://github.com/logdns/imyemail-cloud/releases/latest)；也可直接选择平台开发包：

| 平台 | 下载 |
| --- | --- |
| iOS/iPadOS arm64 | [unsigned IPA](https://github.com/logdns/imyemail-cloud/releases/latest/download/imyemail-cloud-ios-arm64-20261002-unsigned.ipa)（需使用有效 Apple 证书和 provisioning profile 自行签名） |
| Android | [unsigned APK](https://github.com/logdns/imyemail-cloud/releases/latest/download/imyemail-cloud-android-20261002-unsigned.apk)（供开发或自行签名，不能直接作为正式商店安装包） |
| macOS Apple Silicon | [unsigned ZIP](https://github.com/logdns/imyemail-cloud/releases/latest/download/imyemail-cloud-macos-arm64-20261002-unsigned.zip) |
| Linux x86_64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/latest/download/imyemail-cloud-linux-x86_64-20261002.deb) |
| Linux ARM64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/latest/download/imyemail-cloud-linux-arm64-20261002.deb) |
| Windows x64 | [unsigned installer](https://github.com/logdns/imyemail-cloud/releases/latest/download/imyemail-cloud-windows-x64-20261002-setup.exe) |
| Windows ARM64 | [unsigned installer](https://github.com/logdns/imyemail-cloud/releases/latest/download/imyemail-cloud-windows-arm64-20261002-setup.exe) |

完整性校验：[SHA256SUMS](https://github.com/logdns/imyemail-cloud/releases/latest/download/SHA256SUMS)。这些是未签名开发包，平台签名、公证、真机与商店验收边界见 [发布说明](docs/RELEASE.md)。

## 目录

| 目录 | 内容 |
| --- | --- |
| `core/` | Rust IMAP、SMTP、MIME、同步、SQLite、HTML 净化和原生接口 |
| `apple/` | macOS、iOS、iPadOS SwiftUI 客户端 |
| `android/` | Android Jetpack Compose 客户端 |
| `linux/` | Linux GTK4/libadwaita 客户端 |
| `windows/` | Windows WinUI 3 客户端 |
| `push-bridge/` | 可选的签名 Webhook 推送桥 |
| `email-testkit/` | 隔离的 IMAP/SMTP 测试夹具 |
| `brand/` | 客户端品牌源文件和生成脚本 |
| `docs/` | 架构、开发、安全与发布文档 |

## 快速开始

共享核心：

```bash
cd core
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --locked --release -p chck-cli -p chck-ffi-c
```

各客户端的依赖和命令见对应 README：[Apple](apple/README.md)、[Android](android/README.md)、[Linux](linux/README.md)、[Windows](windows/README.md)。开发流程见 [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)，架构见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)，发布说明见 [docs/RELEASE.md](docs/RELEASE.md)。

## 发布边界

CI 会测试共享核心和各平台工程，并生成开发产物。GitHub Releases 提供未签名开发包的公开下载，并在标题和说明中明确标记；它们不等于 Apple 公证包、签名 Android AAB、签名 Windows 安装包或应用商店版本。

## 参与和安全

提交前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 和 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。安全问题请按 [SECURITY.md](SECURITY.md) 私下报告，不要在公开 Issue 中披露尚未修复的漏洞。

## License

[MIT](LICENSE)
