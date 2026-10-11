# 开发指南

`imyemail-cloud-native 0.2.4` 和 `imyemail-cloud-mygo 0.3.4` 独立维护；[安装/下载](INSTALL.zh-CN.md)、[验签](SIGNING.md)、[历史源码恢复与后续更新](VERSIONS.md)。共享核心变更须两条 CI 回归，发布私钥不进入开发/PR 环境。发布前运行 `python3 scripts/test-edition-names.py`，锁住命名、旧安装升级身份、独立数据空间和 DEB 迁移边界。

## 基本要求

- Rust 以 `core/rust-toolchain.toml` 为准。
- Apple 需要当前 Xcode/Swift；Android 需要 JDK 17、SDK 35、NDK 27.2 和 Python 3.11；Windows 需要 .NET 9 和 Windows App SDK；Linux GTK 构建需要 GTK4/libadwaita/WebKitGTK/libsecret。
- 不要把 `.env`、邮箱凭据、真实邮件数据库、签名私钥/P12、商店密钥或生成安装包提交到 Git；`signing/` 的固定公开证书/公钥是验签所需材料，不属于私钥禁入范围。

## 核心门禁

```bash
cd core
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --locked --release -p chck-cli -p chck-ffi-c
```

## 平台门禁

```bash
# Apple
cd apple
swift test -Xswiftc -warnings-as-errors
swift build
./scripts/package-app.sh

# Android
cd android
./gradlew :core:common:test :core:data:test :feature:thread:testDebugUnitTest \
  :app:lintDebug :app:assembleDebug --no-daemon

# Linux 基础；完整 GTK 构建在 Linux CI
cd linux
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked

# Windows 可移植核心；WinUI 构建在 Windows
cd windows
dotnet test Chck.Mail.Tests/Chck.Mail.Tests.csproj -c Release
```

## 改动规则

[双版本开发与发布经验](DEVELOPMENT-LESSONS.md)总结目录迁移、兼容身份、Pulse UI、隔离夹具、签名预检、CI 并发、下载复验和双 cask 防错；版本/工具链以当前清单为准，不复制一次性环境假设。

MyGo 桌面版独立使用 Go 1.27.1、Rust 1.95.0。入口、原生 UI 测试、系统凭据要求和六架构打包命令见 [desktop-mygo/README.md](../desktop-mygo/README.md)；不要把它与旧客户端的系统依赖或版本混用。

1. 协议、同步、存储和 HTML 净化问题优先在共享核心修复，并添加回归测试。
2. UI 保持平台原生；Preview 只能在显式预览模式使用。
3. 涉及 HTML/WebView、凭据、TLS、FFI/JNI、附件或发布脚本时，必须检查对应信任边界。
4. 更新用户可见字符串后运行 `python3 scripts/export-client-locales.py --check`。
5. 构建产物必须来自最终源码，记录版本、架构、SHA-256 和签名状态。

## 日常收口

仅改 Native/MyGo UI 时优先对应门禁；共享核心/契约变更需两版回归，Windows MailKit 路径另核对。发布前运行根目录的 `scripts/test-edition-names.py`、`scripts/test-release-signing.py`，以及双语文档/截图相对链接检查；这些夹具不代替 OS 信任、GPU 或真实邮箱验收。

更新当前指南、版本/安装入口和日期证据，不改写历史记录。普通文档/skill 总结只做内容和结构校验，不自动提交/推送、改标签、dispatch 发布或引入完整安全审计。退出时记录未验收项，只清理本轮临时状态，不删除历史包、签名、截图或失败决策证据。
