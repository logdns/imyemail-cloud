# 安装 imyemail-cloud

[English](INSTALL.md) · [简体中文](INSTALL.zh-CN.md) · [最新版本](https://github.com/logdns/imyemail-cloud/releases/latest)

## macOS（Apple Silicon，macOS 14+）

公开 Homebrew 源将应用安装到 `/Applications`，并链接命令行程序。每台新机器首次执行：

```bash
brew tap logdns/imyemail-cloud
brew trust --tap logdns/imyemail-cloud
brew install imyemail-cloud
```

Homebrew 7 要求首次信任第三方 tap；之后只需 `brew install imyemail-cloud`。更新用 `brew update && brew upgrade imyemail-cloud`；卸载应用及命令行链接用 `brew uninstall imyemail-cloud`。正常卸载不会删除邮件数据或 Keychain 凭据。

[tap 源码](https://github.com/logdns/homebrew-imyemail-cloud)固定 Release ZIP 与 SHA-256；也可以从[最新版本](https://github.com/logdns/imyemail-cloud/releases/latest)手动下载 macOS ZIP。当前包采用 ad-hoc 签名，**未经过 Apple 公证**；首次启动可能需要在“系统设置 → 隐私与安全性”中允许打开。校验 SHA-256 不等于 Apple 公证。

## 其他平台

从[最新版本](https://github.com/logdns/imyemail-cloud/releases/latest)选择对应安装包：

| 平台 | 文件类型 | 限制 |
| --- | --- | --- |
| iOS/iPadOS arm64 | IPA | 未签名；安装前需要自行配置 Apple 证书和 provisioning profile，不能直接用于 TestFlight/App Store。 |
| Android | APK | 未签名 Release 输入；真机或商店分发前需要签名。 |
| Linux x86_64 / ARM64 | DEB | 未进行发行仓库签名。 |
| Windows x64 / ARM64 | EXE 安装器 | 未做 Authenticode 签名。 |

请从安装包**所在的同一标签版本**下载 `SHA256SUMS`：macOS 执行 `shasum -a 256 -c SHA256SUMS`，Linux 执行 `sha256sum -c SHA256SUMS`。总清单包含所有附件；若只下载部分附件，缺失文件会显示失败，此时使用同一版本对应的单文件 `.sha256`。不要将 Latest 包与其他版本的校验清单混用。更多签名和设备验收边界见[发布流程](RELEASE.md)。
