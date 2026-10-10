# MyGo 桌面重构版

`desktop-mygo/` 基于 [egoist/mygo](https://github.com/egoist/mygo) 原生 Go UI，复用 Rust 核心。当前为 `0.3.4` / `mygo-v0.3.4` 自签预览；原版 `0.2.4` 独立更新。历史 `v0.2.0`、`v0.2.1`、`mygo-v0.3.0` 及本地归档不覆盖。见[统一安装](INSTALL.zh-CN.md)、[验签](SIGNING.md)、[版本保留](VERSIONS.md)。

## 当前能力

### Pulse 风格的邮件界面

界面借鉴 [Pulse](https://pulse.egoist.dev/) 的浅灰侧栏、蓝色选中态、彩色线性图标、克制标题和大圆角状态卡片；实际实现为自己的 Go/MyGo 组件，不复制 Pulse 品牌、截图、图标或系统监控功能。主区分为缓存统计、邮件列表、阅读卡片；统计明确是已加载数据，不表示服务器全部邮件或实时同步。深色模式跟随系统，880px 窄窗口折叠统计以保留操作空间；无 HTML/网页 UI。真实合成邮件截图见 [README](../README.zh-CN.md) 和[截图来源](screenshots/README.md)。

支持手动配置 IMAP/SMTP、应用密码、多个账号与文件夹选择、同步、缓存收件箱、按主题/发件人过滤、纯文本阅读、草稿和发送。界面使用英文，操作流程：

1. `Add account`：填写邮箱、应用密码、IMAP/SMTP 主机与端口。证书校验始终开启；587 通常需要勾选 SMTP STARTTLS。
2. `Save account`：密码进入系统钥匙库，不保存在命令行或 SQLite 中。选择侧栏账号和文件夹，点击 `Sync folder`。
3. 点击邮件阅读纯文本；HTML、脚本和远程图片不会执行或加载。
4. `Compose`：使用当前选中账号，填写收件人、主题和正文，保存草稿或发送。失败或状态不明时不会提示已送达，也不应重复点击发送。

尚未覆盖 OAuth、附件导入/导出、HTML 富文本阅读/编辑、后台通知、移动端 UI 和自动更新；不能把这份预览版描述为全部旧客户端的等价替代。

## 安装与安全边界

下载：[MyGo 0.3.4 自签预览](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.4)，六架构/8 包、独立哈希及签名清单；[原生版 0.2.4](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.4) 独立下载。旧版仍可下载。

| 平台 | 架构 | 附件文件名 |
| --- | --- | --- |
| macOS 14+ | Apple Silicon | `imyemail-cloud-mygo-0.3.4-darwin-arm64-selfsigned.zip` |
| macOS 14+ | Intel | `imyemail-cloud-mygo-0.3.4-darwin-amd64-selfsigned.zip` |
| Windows | x64 | `imyemail-cloud-mygo-0.3.4-windows-amd64-selfsigned.zip` |
| Windows | ARM64 | `imyemail-cloud-mygo-0.3.4-windows-arm64-selfsigned.zip` |
| Linux | x86_64 | `imyemail-cloud-mygo-0.3.4-linux-amd64.deb` / `.tar.gz` |
| Linux | ARM64 | `imyemail-cloud-mygo-0.3.4-linux-arm64.deb` / `.tar.gz` |

- macOS arm64/x86_64：固定证书自签 ZIP，未公证；首次启动遵循 Gatekeeper。可单独 `brew install imyemail-cloud-mygo`，首次 tap/trust 见统一安装文档。
- Windows x64/ARM64：UI/核心有自签 Authenticode，ZIP 本身通过签名清单认证，非默认公信；保留同目录核心。
- Linux x86_64/ARM64：Ubuntu 24.04 构建的 DEB 与 tar.gz，需要兼容系统、GTK 3、D-Bus 和已解锁的 Secret Service。上游打包器的 DEB 还声明 WebKitGTK 依赖，但本界面不渲染邮件 HTML。

Linux DEB 可通过 `sudo apt install ./对应文件.deb` 安装；便携 tar.gz 整体解压后，直接运行其中的 `imyemail-cloud-mygo`，保留同目录核心与许可证。便携下载不包含上游单独生成的 install/uninstall 脚本；不要用旧版或不匹配版本的脚本安装它。

数据位于系统用户配置目录下独立的 `imyemail-cloud-mygo/`，不自动迁移或修改旧数据库。账号密码使用 macOS Keychain、Windows 系统凭据或 Linux Secret Service；钥匙库不可用时返回错误，不静默转为明文文件。私有 stdin/stdout 传递 JSON，不开放本地 HTTP 端口。请求/响应有大小限制，操作超时会终止核心进程并要求重新启动。

下载时使用对应标签下的 `.sha256` 或 `SHA256SUMS` 核验。系统签名、公证、真实邮箱、真机 GUI、长期升级与数据恢复不等同于 CI 编译或无窗口测试通过。

按 [SIGNING.md](SIGNING.md) 固定完整指纹、验证清单签名，再核对包哈希。Linux 是签名清单，非 APT 仓库签名。不要关闭系统安全功能。`brew install imyemail-cloud-native` 安装原生版，`brew install imyemail-cloud-mygo` 独立安装预览版。

源码、精确依赖与构建说明见 [desktop-mygo/README.md](../desktop-mygo/README.md)。旧版安装入口见 [INSTALL.zh-CN.md](INSTALL.zh-CN.md)。
