# MyGo 桌面重构版

本次新增 `desktop-mygo/`，基于 [egoist/mygo](https://github.com/egoist/mygo) 的原生 Go UI，复用原有 Rust 邮件核心。桌面版本为 `0.3.0`，采用独立的 `mygo-v0.3.0` 预览发布标签；原来的 `v0.2.1`、移动端、Homebrew cask 和本地归档保持不变。

## 当前能力

支持手动配置 IMAP/SMTP、应用密码、多个账号与文件夹选择、同步、缓存收件箱、按主题/发件人过滤、纯文本阅读、草稿和发送。界面使用英文，操作流程：

1. `Add account`：填写邮箱、应用密码、IMAP/SMTP 主机与端口。证书校验始终开启；587 通常需要勾选 SMTP STARTTLS。
2. `Save account`：密码进入系统钥匙库，不保存在命令行或 SQLite 中。选择侧栏账号和文件夹，点击 `Sync folder`。
3. 点击邮件阅读纯文本；HTML、脚本和远程图片不会执行或加载。
4. `Compose`：使用当前选中账号，填写收件人、主题和正文，保存草稿或发送。失败或状态不明时不会提示已送达，也不应重复点击发送。

尚未覆盖 OAuth、附件导入/导出、HTML 富文本阅读/编辑、后台通知、移动端 UI 和自动更新；不能把这份预览版描述为全部旧客户端的等价替代。

## 安装与安全边界

下载入口：[MyGo 0.3.0 桌面预览版](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.0)。只有发布门禁完成后附件才可用；旧版 [v0.2.1](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.1) 仍为独立下载入口。

| 平台 | 架构 | 附件文件名 |
| --- | --- | --- |
| macOS 14+ | Apple Silicon | `imyemail-cloud-mygo-0.3.0-darwin-arm64-adhoc.zip` |
| macOS 14+ | Intel | `imyemail-cloud-mygo-0.3.0-darwin-amd64-adhoc.zip` |
| Windows | x64 | `imyemail-cloud-mygo-0.3.0-windows-amd64-unsigned.zip` |
| Windows | ARM64 | `imyemail-cloud-mygo-0.3.0-windows-arm64-unsigned.zip` |
| Linux | x86_64 | `imyemail-cloud-mygo-0.3.0-linux-amd64.deb` / `.tar.gz` |
| Linux | ARM64 | `imyemail-cloud-mygo-0.3.0-linux-arm64.deb` / `.tar.gz` |

- macOS arm64/x86_64：应用 ZIP，ad-hoc 签名，未公证；首次启动遵循系统 Gatekeeper。
- Windows x64/ARM64：未做 Authenticode 的便携 ZIP，解压后保留同目录的核心程序。
- Linux x86_64/ARM64：DEB 与 tar.gz，需要 GTK 3 和已解锁的 Secret Service。

数据位于系统用户配置目录下独立的 `imyemail-cloud-mygo/`，不自动迁移或修改旧数据库。账号密码使用 macOS Keychain、Windows 系统凭据或 Linux Secret Service；钥匙库不可用时返回错误，不静默转为明文文件。私有 stdin/stdout 传递 JSON，不开放本地 HTTP 端口。请求/响应有大小限制，操作超时会终止核心进程并要求重新启动。

下载时使用对应标签下的 `.sha256` 或 `SHA256SUMS` 核验。系统签名、公证、真实邮箱、真机 GUI、长期升级与数据恢复不等同于 CI 编译或无窗口测试通过。

macOS/Linux 使用 `shasum -a 256 文件名` 或 `sha256sum -c 文件名.sha256`；Windows PowerShell 使用 `Get-FileHash 文件名 -Algorithm SHA256`，与同一 Release 中的完整哈希比较。不要为运行预览版关闭系统安全功能。Homebrew 的 `brew install imyemail-cloud` 仍安装原版，不会切换到 MyGo。

源码、精确依赖与构建说明见 [desktop-mygo/README.md](../desktop-mygo/README.md)。旧版安装入口见 [INSTALL.zh-CN.md](INSTALL.zh-CN.md)。
