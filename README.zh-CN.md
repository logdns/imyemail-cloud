# imyemail-cloud

[English](README.md) · [简体中文](README.zh-CN.md) · [文档](docs/README.md)

## 看界面，选择下载

| **imyemail-cloud-native · 0.2.4 原生版** | **imyemail-cloud-mygo · 0.3.4 桌面预览版** |
| --- | --- |
| [![原生客户端](docs/screenshots/imyemail-cloud-native.png)](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.4) | [![MyGo 客户端](docs/screenshots/imyemail-cloud-mygo.png)](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.4) |
| 平台原生 UI：macOS、iOS/iPadOS、Android、Linux、Windows；延续现有客户端。 | Go/MyGo 原生 UI：macOS、Windows、Linux；借鉴 [Pulse](https://pulse.egoist.dev/) 的侧栏、蓝色选中态、圆角卡片和明暗风格，使用自己的组件与图标。 |
| **[下载原生版](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.4)** · `brew install imyemail-cloud-native` | **[下载 MyGo](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.4)** · `brew install imyemail-cloud-mygo` |

真实 UI 的合成邮件截图：原生版为 macOS Preview，MyGo 为实际 UI 无窗口渲染器，不含真实邮箱，不是 AI 效果图。[截图来源及深色界面](docs/screenshots/README.md)。选择系统/CPU 下载即可，不必编译源码。两版免费、MIT 开源，账号、数据与更新独立；MyGo 使用英文界面与纯文本阅读，不等于原生版全部功能。

## 下载与安装

| 系统 | 原生版 0.2.4 | MyGo 0.3.4 |
| --- | --- | --- |
| macOS Apple Silicon | [自签 ZIP](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.4/imyemail-cloud-native-macos-arm64-20261011-selfsigned.zip) | [自签 ZIP](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.4/imyemail-cloud-mygo-0.3.4-darwin-arm64-selfsigned.zip) |
| macOS Intel | 不提供 | [自签 ZIP](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.4/imyemail-cloud-mygo-0.3.4-darwin-amd64-selfsigned.zip) |
| Windows x64 | [自签安装器](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.4/imyemail-cloud-native-windows-x64-20261011-selfsigned-setup.exe) | [自签便携 ZIP](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.4/imyemail-cloud-mygo-0.3.4-windows-amd64-selfsigned.zip) |
| Windows ARM64 | [自签安装器](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.4/imyemail-cloud-native-windows-arm64-20261011-selfsigned-setup.exe) | [自签便携 ZIP](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.4/imyemail-cloud-mygo-0.3.4-windows-arm64-selfsigned.zip) |
| Linux x86_64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.4/imyemail-cloud-native-linux-x86_64-20261011.deb) | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.4/imyemail-cloud-mygo-0.3.4-linux-amd64.deb) / [tar.gz](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.4/imyemail-cloud-mygo-0.3.4-linux-amd64.tar.gz) |
| Linux ARM64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.4/imyemail-cloud-native-linux-arm64-20261011.deb) | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.4/imyemail-cloud-mygo-0.3.4-linux-arm64.deb) / [tar.gz](https://github.com/logdns/imyemail-cloud/releases/download/mygo-v0.3.4/imyemail-cloud-mygo-0.3.4-linux-arm64.tar.gz) |
| Android | [固定密钥签名 APK](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.4/imyemail-cloud-native-android-20261011-selfsigned.apk) | 不提供 |
| iOS/iPadOS arm64 | [unsigned IPA](https://github.com/logdns/imyemail-cloud/releases/download/native-v0.2.4/imyemail-cloud-native-ios-arm64-20261011-unsigned.ipa)，仍须 Apple provisioning | 不提供 |

### Homebrew（macOS 14+）

```bash
brew tap logdns/imyemail-cloud
brew trust --tap logdns/imyemail-cloud
brew install imyemail-cloud-native
brew install imyemail-cloud-mygo
```

选择一版，或在 Apple Silicon 同时安装。Homebrew 7 首次信任第三方 tap 一次；分别使用 `brew upgrade imyemail-cloud-native` 或 `brew upgrade imyemail-cloud-mygo` 更新。[双版本公开 tap](https://github.com/logdns/homebrew-imyemail-cloud) 独立固定版本、URL 和哈希。

桌面自签包**不代表 Apple 公证、默认公信、SmartScreen 信誉或商店发布**；Linux 使用签名校验清单，不是 APT 仓库签名。下载后先核对固定证书指纹、验签 `SHA256SUMS` 并复算包哈希。iOS IPA 不是直接安装包，需有效 Apple 签名与配置。MyGo ZIP/tar.gz 需保留内置 Rust helper 和许可证。

[各平台安装、校验、卸载与升级](docs/INSTALL.zh-CN.md) · [签名说明](docs/SIGNING.md) · [版本、数据兼容与回滚](docs/VERSIONS.md)。旧 Homebrew 用户先关闭 app、备份数据库与 Keychain，禁用 cleanup 卸载旧 `imyemail-cloud`（不用 `--zap`），再装 `imyemail-cloud-native`；旧/新原生共享数据，不同时运行。

## 独立维护与旧版

公开仓库保持 [logdns/imyemail-cloud](https://github.com/logdns/imyemail-cloud)，产品帮助链接为 [https://imy.email](https://imy.email)。原生版使用 `native-v*`，MyGo 使用 `mygo-v*`；GitHub Latest 属于原生版。无收费、软件激活、设备名额或 Web 程序。

历史 [v0.2.1](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.1)、[v0.2.0](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.0)、[mygo-v0.3.0](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.0) 的名称、字节、签名、哈希和标签不重写；原始 Chck 资料保存在本地被忽略的 `chckemail-old/`，不公开。

## 开发与安全

`core/` 为共享 Rust 邮件核心；`apple/`、`android/`、`linux/`、`windows/` 为平台原生客户端；`desktop-mygo/` 为 Go 桌面版；`push-bridge/`、`email-testkit/` 为可选服务和隔离测试工具。内部兼容标识不批量改动。

[架构](docs/ARCHITECTURE.md) · [开发门禁](docs/DEVELOPMENT.md) · [MyGo 能力边界](docs/MYGO.zh-CN.md) · [发布 SOP](docs/RELEASE.md) · [验证记录](artifacts/20261011-self-signed-tracks/verification.md)。共享核心修复必须两版回归，分别发布递增版本并更新对应 cask 与双语文档。截图与 CI 不代替真实邮箱收发、系统钥匙库 ACL、长期升级恢复或真机验收。

参与贡献请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 与 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。安全问题按 [SECURITY.md](SECURITY.md) 私下报告，不公开未修复漏洞。

## License

[MIT](LICENSE)。第三方依赖保留各自许可证。
