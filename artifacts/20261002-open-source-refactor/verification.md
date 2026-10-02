# imyemail-cloud 开源重构与发布验证

## 范围与环境

- 日期：2026-10-02，Asia/Shanghai；最后本地复核时间 12:24 CST（+0800）。
- 主机：macOS 27.0.1（Build 26A434），Apple Silicon arm64。
- 工具链：Rust/Cargo 1.95.0、Swift 6.4、Xcode 27.0、.NET SDK 9.0.318。
- 源码状态：从旧仓库提交 `38367504a777b2cfcc6bee600e790954505f943a` 的工作树重构；本记录随新的 `imyemail-cloud` 初始提交发布，不继承旧 Git 历史。
- 审查模式：针对品牌、商业授权移除、网站移除、凭据/HTML/WebView/JNI 和发布边界的聚焦 source-first 审查，不是完整安全审计。
- 执行隔离：目标代码仅在 `/private/tmp/imyemail-cloud-verify.5nrUS1` 的一次性源码副本中执行；空白环境白名单、外网禁用、仅 localhost、仅临时根可写，并设置 CPU、进程、文件大小和墙钟限制。没有保留原始日志、凭据、邮箱数据库或 `.env`。

## 旧版归档

- 本地归档：`chckemail-old/`，约 6.9 GB、30,596 个文件；由根 `.gitignore` 排除，不进入公开仓库。
- 完整历史：`chckemail-old/chckemail-original.bundle`。
- `git bundle verify`：退出码 0；bundle 包含旧 `main`、远程引用和 `HEAD`，并记录完整历史。
- 原提交：`38367504a777b2cfcc6bee600e790954505f943a`；原远程：`https://github.com/logdns/chckemail.git`。

## 实现与一致性结果

- 对外产品名统一为 `imyemail-cloud`，客户端 Bundle/Application ID 为 `email.imy.cloud`，客户端主页/帮助链接为 `https://imy.email`，公开源码链接为 `https://github.com/logdns/imyemail-cloud`。
- 删除官网/Web 应用、商店营销资料、支付/订阅、软件激活、设备名额和商业授权门禁；`chck-license` 已从源码、workspace 和 Cargo lock 中移除。
- 保留第三方许可证，以及邮件服务商要求的“应用密码/授权码”说明；后者是 IMAP/SMTP 凭据，不是软件许可证。
- 内部 `chck-*` crate、`Chck.Mail.*` 命名空间、Swift/Kotlin 类型和 C ABI 名称作为工程/ABI 兼容标识保留，不作为用户可见品牌。
- 修复品牌迁移发现的 Android 断点：通知图标文件改为 `ic_stat_imyemail_cloud.xml`；Rust JNI 符号和 SecretStore 类路径同步到 `email.imy.cloud.chckcore`；CI 增加静态边界检查。
- 首轮 GitHub Linux 完整特性 Clippy 发现设置动作遗留一个未使用的 `bridge` 克隆；已删除并重跑 CI。
- 六种语言、643 条词典、9 份离线资源一致；版本统一为 `0.2.0`。

## 本地门禁结果

以下命令均针对最终源码，除明确注明的受限项外退出码为 0：

- `git diff --check`。
- `python3 scripts/export-client-locales.py --check`：643 entries × 6 languages；9 offline resources。
- `cargo fmt --all -- --check`：core、push-bridge、linux。
- Core `cargo clippy --locked --workspace --all-targets -- -D warnings`。
- Core `cargo test --locked --workspace -- --skip keychain_backend_roundtrip --skip apple_os_secrets_skip_sidecar --skip apple_open_stores_password_in_keychain_not_sidecar`：117 项通过、0 失败；其中隔离 IMAP/SMTP/TLS 回归通过。
- Core `cargo build --locked --release -p chck-cli -p chck-ffi-c`。
- Push Bridge：Clippy、2 项测试及 Release build 通过。
- Linux 默认特性：Clippy、11 项测试及 Release build 通过。macOS 生成的该二进制只是跨平台逻辑构建证据，不是 Linux GTK/DEB 产物。
- Windows 可移植核心：`dotnet test Chck.Mail.Tests/Chck.Mail.Tests.csproj -c Release`，133 项通过、0 失败；恢复仅使用临时 NuGet 缓存，外网审计源被隔离并产生预期 `NU1900` 警告。WinUI 和安装器未在 macOS 构建。
- Workflow YAML 由 Ruby YAML parser 解析通过；Release workflow 校验多端版本并强制 GitHub prerelease，不覆盖已有 tag/release。

## 临时开发产物

以下均为 macOS arm64、未签名、位于一次性隔离目录的开发构建，不属于公开发布附件：

| 产物 | SHA-256 |
| --- | --- |
| Core CLI `imyemail-cloud` | `619e371c9b7a34ce49e8c655dc192653c13ab7a2aaac43438e711c72624c8a35` |
| Core C ABI `libchck_mail.a` | `037a88bf4543c6ead805660c0ce0c464abae55abe48660c028cec25a7d0fad20` |
| Core C ABI `libchck_mail.dylib` | `e5f1d70370602916def5727c096ec82efa1ea5170c0c279aa8b31f0a1dd483bc` |
| Push Bridge | `6e57a6dd2cfaeea3db1332c1bf98171ac7fc85d902e45dd900c0b06250b67a5b` |
| Linux crate 的 macOS 默认特性构建 | `53ad18e3b337d41fa9addb9a286af1e8f5ca249e585bce2f5c1f626307237b6c` |

## 聚焦安全检查

- 搜索未发现旧公开域名、旧 GitHub URL、`chck-website`、商业 receipt、软件授权门禁或 `chck-license` 残留。`stripe` 的剩余命中是邮件列表账号色条变量/CSS，不是 Stripe 支付实现。
- 新公开树没有网站/Web 应用目录。邮件 HTML 净化和平台 WebView 代码仍然存在，因为它们用于安全显示不可信邮件正文，不属于被删除的网站程序。
- 凭据继续进入 Keychain/Keystore/Credential Locker/libsecret 或受限 sidecar，而不是 SQLite；邮箱应用密码/授权码提示保留。
- 发布工作流明确区分未签名开发包与 Apple 公证、Android/Windows 签名和商店发布。聚焦检查未发现确认的授权残留或由本次重构引入的确认边界漏洞；这不构成“无漏洞”声明。

## 未在本机验收的边界

- 三项需要访问宿主 macOS Keychain 的测试：`keychain_backend_roundtrip`、`apple_os_secrets_skip_sidecar`、`apple_open_stores_password_in_keychain_not_sidecar`。隔离策略禁止写宿主 Keychain，因此明确跳过，交由 GitHub macOS CI 验证。
- Apple Swift build/test：`xcrun`/XCBuild 强制写 `/var/folders` 系统缓存，被隔离策略阻止；未证明源码失败，也不能写成本地通过。Swift 测试、macOS 包和 iOS Simulator app 已由 GitHub macOS runner 验证通过。
- Android：本机只有 JDK 25.0.2，而 AGP 工程要求 JDK 17；Gradle 在配置阶段以 `25.0.2` 失败。Temurin 17 GitHub runner 上的 JNI、单测、lint、Debug APK 和 unsigned Release APK 已验证通过。
- Windows WinUI/XAML、x64/ARM64 self-contained publish 和 Inno Setup 安装器需要 Windows runner；Linux GTK/WebKitGTK、DEB 安装/启动/卸载需要 Linux runner。这些项目均已在对应 GitHub runner 验证通过。
- iOS/Android 真机、真实邮箱收发、Apple 公证、Android/Windows 正式签名、商店审核及生产推送部署均未在本机验收。

## 发布闭环

- `.github/workflows/ci.yml` 是多平台合并门禁。
- `.github/workflows/release-build.yml` 从通过验证的源码构建多平台未签名开发附件、生成 `SHA256SUMS` 并创建 `v0.2.0` GitHub prerelease。
- 公开仓库：<https://github.com/logdns/imyemail-cloud>；默认分支 `main`；主页 <https://imy.email>。
- 初始开源提交：`8e7d3596b5b78e868b12303520dbdb8a6eba8214`；Linux 完整特性修复及实际发布源提交：`9e74649501657c60b070f36b2dd34bce5a01dff1`。
- 多平台 CI：<https://github.com/logdns/imyemail-cloud/actions/runs/36962934681>，10/10 jobs 成功。
- Release workflow：<https://github.com/logdns/imyemail-cloud/actions/runs/36963674864>，7/7 jobs 成功。
- GitHub prerelease：<https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.0>；非 draft、`isPrerelease=true`，目标提交为 `9e74649501657c60b070f36b2dd34bce5a01dff1`。
- 发布附件包括 Android APK、Linux x86_64/ARM64 DEB、macOS arm64 ZIP、Windows x64/ARM64 EXE，以及六份独立 `.sha256` 和总 `SHA256SUMS`，共 13 个文件。下载全部 322 MB 附件后，`SHA256SUMS` 的 12 项以及六份独立校验均复算通过。
- 所有附件均为未签名开发包；Apple 公证、Android/Windows 正式签名、真机和商店验收仍属于明确的生产发布边界。

## 清理

- 新仓库不跟踪本地归档、构建目录、签名材料、安装包、数据库或敏感日志。
- 完成本地验证后删除一次性 `/private/tmp/imyemail-cloud-verify.5nrUS1`；该目录只含可重建源码副本、依赖缓存和构建产物。
