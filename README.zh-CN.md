# imyemail-cloud

[English](README.md) · [简体中文](README.zh-CN.md) · [中文文档导航](docs/README.md) · [最新版本下载](https://github.com/logdns/imyemail-cloud/releases/latest)

## 看界面，选择下载

| **imyemail-cloud-native · 0.2.2 原生版** | **imyemail-cloud-mygo · 0.3.1 桌面预览版** |
| --- | --- |
| [![原生客户端界面](docs/screenshots/imyemail-cloud-native.png)](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.2) | [![MyGo 客户端界面](docs/screenshots/imyemail-cloud-mygo.png)](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.1) |
| 平台原生 UI：macOS、iOS/iPadOS、Android、Linux、Windows；希望继续使用原有客户端请选择此版。 | 使用 [egoist/mygo](https://github.com/egoist/mygo) 的 Go 原生桌面 UI：macOS、Windows、Linux，目前英文界面、纯文本邮件，功能较少。 |
| **[下载原生版](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.2)** · `brew install imyemail-cloud-native` | **[下载 MyGo](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.1)** · `brew install imyemail-cloud-mygo` |

真实 UI 的合成邮件截图：原生版为 macOS Preview，MyGo 为实际 UI 无窗口渲染器，不含真实邮箱，不是 AI 效果图。[截图来源](docs/screenshots/README.md)。选择系统/CPU 下载即可，不必编译源码。两版免费、MIT 开源、独立账号数据及更新；MyGo 不等于原生版全部功能。桌面自签包**未获 Apple 公证/默认公信**，iOS IPA 仍须 Apple provisioning。

imyemail-cloud 是 MIT 开源的多端原生邮件客户端，公开仓库为 [github.com/logdns/imyemail-cloud](https://github.com/logdns/imyemail-cloud)，产品链接为 [https://imy.email](https://imy.email)。**imyemail-cloud-native 0.2.2** 与 **imyemail-cloud-mygo 0.3.1 桌面预览版**独立保留、维护和发布。

本仓库只包含客户端、共享邮件核心、可选推送桥、测试工具、构建脚本和开源文档。它不包含官网/Web 应用、支付、订阅、软件激活、设备名额或商业授权门禁；同步和发送等客户端功能不需要购买软件许可证。

## MyGo 桌面预览版

新增 [MyGo 0.3.1 自签发布](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.1)，基于 [egoist/mygo](https://github.com/egoist/mygo) 原生 Go UI 与同次构建 Rust 核心，macOS/Windows/Linux x64/ARM64。它不替代原版全部功能；移动端、源码、数据和独立 Homebrew 包名保留。见[安装](docs/INSTALL.zh-CN.md)、[签名校验](docs/SIGNING.md)、[后续更新与回滚](docs/VERSIONS.md)。历史 [v0.2.1](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.1)、[v0.2.0](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.0)、[mygo-v0.3.0](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.0) 标签和附件不覆盖。

## 下载

[原生版 0.2.2 自签包](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.2) · [MyGo 0.3.1 自签包](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.1)。Latest 属于原生版，MyGo 必须使用独立入口。

### Homebrew（macOS Apple Silicon）

首次使用时信任公开 tap，然后使用短包名安装：

```bash
brew tap logdns/imyemail-cloud
brew trust --tap logdns/imyemail-cloud
brew install imyemail-cloud-native
brew install imyemail-cloud-mygo
```

Homebrew 7 要求首次信任第三方 tap；之后使用 `brew upgrade imyemail-cloud-native` 更新。各平台安装、校验、卸载及签名限制见[安装指南](docs/INSTALL.zh-CN.md)。Homebrew 源公开在 [logdns/homebrew-imyemail-cloud](https://github.com/logdns/homebrew-imyemail-cloud)。

| 平台 | 下载 |
| --- | --- |
| iOS/iPadOS arm64 | [unsigned IPA](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.2/imyemail-cloud-native-ios-arm64-20261011-unsigned.ipa)，仍需 Apple provisioning |
| Android | [签名 APK](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.2/imyemail-cloud-native-android-20261011-selfsigned.apk) |
| macOS Apple Silicon | [自签 ZIP](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.2/imyemail-cloud-native-macos-arm64-20261011-selfsigned.zip) 或原生版 Homebrew |
| Linux x86_64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.2/imyemail-cloud-native-linux-x86_64-20261011.deb) |
| Linux ARM64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.2/imyemail-cloud-native-linux-arm64-20261011.deb) |
| Windows x64 | [自签安装器](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.2/imyemail-cloud-native-windows-x64-20261011-selfsigned-setup.exe) |
| Windows ARM64 | [自签安装器](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.2/imyemail-cloud-native-windows-arm64-20261011-selfsigned-setup.exe) |
| MyGo 桌面六架构 | [独立自签包](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.1)，独立应用及 cask |

按 [SIGNING.md](docs/SIGNING.md) 固定指纹验证对应 Release 的 **SHA256SUMS 签名**后再核验包。macOS/Windows 为代码自签，非公信/公证；Linux 为签名清单，非 APT 仓库签名；iOS unsigned。真机、商店验收独立。

### MyGo：选择桌面系统 / CPU

| 系统 | MyGo 0.3.1 下载 |
| --- | --- |
| macOS Apple Silicon | [ZIP](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.1/imyemail-cloud-mygo-0.3.1-darwin-arm64-selfsigned.zip) |
| macOS Intel | [ZIP](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.1/imyemail-cloud-mygo-0.3.1-darwin-amd64-selfsigned.zip) |
| Windows x64 | [便携 ZIP](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.1/imyemail-cloud-mygo-0.3.1-windows-amd64-selfsigned.zip) |
| Windows ARM64 | [便携 ZIP](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.1/imyemail-cloud-mygo-0.3.1-windows-arm64-selfsigned.zip) |
| Linux x86_64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.1/imyemail-cloud-mygo-0.3.1-linux-amd64.deb) / [tar.gz](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.1/imyemail-cloud-mygo-0.3.1-linux-amd64.tar.gz) |
| Linux ARM64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.1/imyemail-cloud-mygo-0.3.1-linux-arm64.deb) / [tar.gz](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.1/imyemail-cloud-mygo-0.3.1-linux-arm64.tar.gz) |

MyGo 没有 Android/iOS 安装包；整体解压，保留 Rust 核心和许可证。原生 macOS 仅提供 Apple Silicon 包，Intel Mac 可选 MyGo。[安装、校验与升级](docs/INSTALL.zh-CN.md)。

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

CI 测试两版并构建；仅手动请求、标签一致的发布接收密钥。桌面自签及 Android APK 签名不等于公信、Apple 公证、真机验收或商店发布；iOS 仍需 Apple provisioning。历史 unsigned 包按原说明保留。

## 参与和安全

提交前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 和 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。安全问题请按 [SECURITY.md](SECURITY.md) 私下报告，不要在公开 Issue 中披露尚未修复的漏洞。

## License

[MIT](LICENSE)
