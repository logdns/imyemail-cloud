# imyemail-cloud v0.2.1 iOS IPA 发布验证

## 范围

- 日期：2026-10-02，Asia/Shanghai。
- 目标：为 GitHub Latest Release 增加 iOS/iPadOS arm64 IPA，并保持源码、标签和附件来自同一发布提交。
- 交付边界：当前环境没有有效 Apple Development/Distribution identity 或 provisioning profile；产物是 unsigned IPA，只能作为自行签名或后续 Apple 发布流程的输入，不声明可直接安装、TestFlight 或 App Store 上架。
- 审查模式：针对 Apple 设备构建、IPA 结构、嵌入式 Rust C ABI、本地路径/密钥泄露和 GitHub 发布链路的聚焦 source-first 检查，不是完整安全审计。

## 实现

- 新增 `apple/scripts/package-ios-device.sh`：构建 `iphoneos/arm64` Release app，验证设备平台、Bundle ID、内置 C ABI 和无签名状态，再生成标准 `Payload/imyemail-cloud.app` IPA。
- CI 与 Release workflow 均构建 IPA；Release 同时上传独立 `.sha256`，总 `SHA256SUMS` 覆盖所有附件。
- 版本由 `0.2.0` 同步提升到 `0.2.1`；不移动或覆盖已发布的 `v0.2.0` 标签。
- 移除 Apple 源码中旧开发机的硬编码绝对路径，改用 bundle、当前工作目录和标准安装目录候选；CI 新增本地用户路径门禁。
- Rust Apple 构建使用 `--remap-path-prefix`，避免将项目工作区和 Cargo 主目录写入发布二进制。

## 本地构建与复验

- 主机：macOS 27.0.1 arm64；Xcode 27.0（iOS SDK 27.0）；Rust 1.95.0。
- `apple/scripts/package-ios-device.sh`：退出码 0。
- IPA：`imyemail-cloud-ios-arm64-20261002-unsigned.ipa`；最终本地复验构建 SHA-256 `8213638501dcfb8ef6b6954d84e5dc10de7ea66f83b887d0f5857e24b7d48d59`。GitHub runner 会从同一源码重新构建，远端附件哈希另行记录。
- 独立解包验证：存在 `Payload/imyemail-cloud.app`，无 `__MACOSX`、`embedded.mobileprovision` 或 `_CodeSignature`。
- 主程序：Mach-O arm64；`LC_BUILD_VERSION` 为 `IOS`，最低 iOS 17.0，SDK 27.0。
- 元数据：Bundle ID `email.imy.cloud`；版本 `0.2.1`；build `3`。
- `nm` 确认 `_chck_mail_open`、`_chck_mail_call`、`_chck_mail_close` 已链接进主程序。
- `unzip -t`、独立 `.sha256` 复算通过；字符串扫描未发现本机工作区绝对路径、私钥头或 GitHub token 标记。
- Apple `swift test -Xswiftc -warnings-as-errors`：53 项执行、0 失败、4 项按现有环境条件跳过。
- Core：fmt、Clippy（warnings denied）及 workspace 测试通过；包含宿主 Keychain 测试、隔离 IMAP/SMTP/TLS 回归。
- Push Bridge：fmt、Clippy、2 项测试通过；Linux 默认特性 fmt、Clippy、11 项测试通过。
- Windows 可移植核心：133/133 项测试通过。
- 三份 Apple plist、Windows Appx manifest、Linux AppStream XML、两份 GitHub workflow YAML、643 × 6 本地化条目和 9 份离线资源验证通过。
- Rust 三个 workspace 的 `Cargo.lock` 与 `--locked` metadata 验证通过；多平台版本元数据统一为 `0.2.1`。

## 聚焦安全结论

- 发现并修复两个打包问题：首次归档缺少顶层 `Payload/`；Apple 源码包含旧开发机绝对路径。两项均在发布前被复验门禁拦截，修复后重建通过。
- 首轮 GitHub 仓库卫生 job 的路径门禁扫描整个 Apple 目录，误命中 IPA 脚本自身用于二进制检查的 `/Users/...` 正则；已将源码门禁限定到 Swift 文件，二进制扫描仍由打包脚本独立执行。
- 未把证书、私钥、provisioning profile、Keychain 或签名密码写入源码、构建产物或 GitHub Actions。
- unsigned IPA 本身不能替代 Apple 签名、公证、真机安装、TestFlight 和 App Store 审核；这些仍是未验收边界。

## 远端发布

- 待记录最终源码提交、CI run、Release workflow、GitHub Latest Release URL、附件清单和远端 SHA-256 复算结果。
