# 安装：选择原版或 MyGo 版

[English](INSTALL.md) · [简体中文](INSTALL.zh-CN.md) · [签名校验](SIGNING.md) · [版本保留与更新](VERSIONS.md)

两条版本线在[原 GitHub](https://github.com/logdns/imyemail-cloud)独立保留和维护，不互相替代，不自动迁移账号或删除邮件。

| 版本线 | 当前下载 | 平台 | 保留历史 |
| --- | --- | --- | --- |
| 原生版 `imyemail-cloud-native` 0.2.2 | [native-v0.2.2 自签包](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.2) | macOS arm64、iOS/iPadOS、Android、Linux/Windows x64/ARM64 | [v0.2.1](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.1)、[v0.2.0](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.0) |
| `imyemail-cloud-mygo` 0.3.1 预览 | [mygo-v0.3.1 自签包](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.1) | macOS/Windows/Linux amd64/arm64 | [mygo-v0.3.0](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.0) |

MyGo 是原生 Go 桌面版，不是 Web 程序，也不等价于原版全部功能；[能力与钥匙库要求](MYGO.zh-CN.md)。GitHub Latest 属于原版，MyGo 使用独立标签入口。

## 先校验

从**同一标签**下载包、对应 `.sha256`、`SHA256SUMS`、`SHA256SUMS.sig`、`release-cert.pem`、`release-public.pem`。按 [SIGNING.md](SIGNING.md) 核对完整证书指纹，验证 RSA/SHA-256 清单签名，再核对包哈希。同处下载的公钥须与独立可信副本/指纹比较。

macOS/Windows 为固定证书代码自签；Android 使用固定应用密钥；Linux 采用签名清单，**不等于 APT 仓库签名**。自签不等于默认信任、Apple 公证、商店审核或真机验收，iOS 仍 unsigned。

## macOS 14+

```bash
brew tap logdns/imyemail-cloud
brew trust --tap logdns/imyemail-cloud
brew install imyemail-cloud-native  # 原生版，仅 Apple Silicon
brew install imyemail-cloud-mygo  # 可选独立 MyGo，Intel / Apple Silicon
```

Homebrew 7 首次须信任第三方 tap。更新：`brew update && brew upgrade 对应包名`；卸载：`HOMEBREW_NO_INSTALL_CLEANUP=1 brew uninstall 对应包名`，避免自动清理其他依赖。正常卸载保留邮件和 Keychain。两个 [cask](https://github.com/logdns/homebrew-imyemail-cloud) 独立固定 URL/哈希，原包名不会切换到 MyGo。

手动安装对应架构 `*-selfsigned.zip`，整体解压，将 `imyemail-cloud-native.app` 或 `imyemail-cloud-mygo.app` 放入 Applications，不覆盖另一版。按 SIGNING.md 检查签名。未经 Apple 公证，Gatekeeper 仍可能拦截；仅确认可信后通过“系统设置 → 隐私与安全性”正常允许，不关闭 Gatekeeper、不全局清除 quarantine，也不保证导入证书后一定能启动。

## Windows

原版选择 x64/ARM64 `*-selfsigned-setup.exe`，核验后安装，需 Windows 10 1809+/11、WebView2。MyGo 选择 `*-windows-amd64-selfsigned.zip` 或 `*-windows-arm64-selfsigned.zip`，整体解压到独立用户目录，运行 `imyemail-cloud-mygo.exe`，保留核心及许可证；这是便携包，不是安装器/MSIX。

自签发布者默认不受信任，SmartScreen 信誉是另一项检查。SIGNING.md 提供固定指纹、可选**当前用户限定**证书导入/移除步骤。不关闭 SmartScreen、不自动导入全局证书、不描述成公信签名。

## Android（仅原版）

`imyemail-cloud-native-android-20261011-selfsigned.apk`：核验哈希及 APK 签名后，按系统提示允许所选下载应用安装。最低 Android 8，包含 arm64-v8a/armeabi-v7a/x86_64 核心，不是 Play Store 包，未声称真机验收。

更新必须沿用**同一签名密钥**并增加 versionCode（0.2.2 为 4）。Debug、自行重签或历史 Chck APK 可能不同签名，不能直接覆盖；先备份，不要为绕过签名不一致直接卸载账号。

## Linux

原生版选 x86_64/ARM64 DEB；MyGo 选 amd64/arm64 DEB 或 tar.gz，Ubuntu 24.04 构建。核验后 `sudo apt install ./对应包.deb`；卸载仅用 `sudo apt remove imyemail-cloud-native` 或 `sudo apt remove imyemail-cloud-mygo`，不用 purge/autoremove。MyGo tar 整体解压，原地运行，保留核心和许可证。

MyGo 需兼容运行库、GTK 3、D-Bus、已解锁 Secret Service，无明文密码回退。上游 DEB 另声明 WebKitGTK，但本 UI 不渲染邮件 HTML。原版 GTK4 依赖独立，不复用其他版本安装脚本。

## iOS/iPadOS（仅原版）

`*-unsigned.ipa` 是设备 arm64 构建/签名输入，不能直接安装。需有效 Apple 证书、匹配 App ID、provisioning profile 和获准设备。普通自签不能代替 Apple provisioning；未发布 TestFlight/App Store。

## 并存、更新、回滚

更新前关闭对应程序，分别备份该版数据库及系统钥匙库凭据，不公开备份。原版 ID `email.imy.cloud`，MyGo ID `email.imy.cloud.mygo`、独立 `imyemail-cloud-mygo` 配置目录；无自动迁移/自动二进制更新。

历史标签/包保留；回滚用旧包及**对应旧清单**，数据库兼容性未知则恢复升级前备份。Android 普通流程拒绝低 versionCode，优先前向修复。恢复源码、独立维护与密钥保留见 [VERSIONS.md](VERSIONS.md)。

### 旧原生版升级到新名称

应用 ID、数据库/钥匙库位置、Windows 安装升级标识不变。先关闭旧原生版，不要让旧/新两个原生 app 同时读写同一数据库；MyGo 独立，不受改名影响。

旧 Homebrew 用户先备份原生数据库和 Keychain，再执行 `HOMEBREW_NO_INSTALL_CLEANUP=1 brew uninstall --cask imyemail-cloud`（不用 `--zap`），然后 `brew install imyemail-cloud-native`。旧 cask 保留为历史入口，与新原生 cask 互斥，不与 MyGo 冲突。Linux 新 DEB 只对旧 `imyemail-cloud (< 0.2.2)` 声明 Breaks/Replaces；确认 APT 计划后安装，不删除邮箱数据。原生内部 CLI/Windows 程序保留 `imyemail-cloud` 文件名用于兼容，新 Homebrew CLI/Linux 别名为 `imyemail-cloud-native`。
