# 开发指南

## 基本要求

- Rust 以 `core/rust-toolchain.toml` 为准。
- Apple 需要当前 Xcode/Swift；Android 需要 JDK 17、SDK 35、NDK 27.2 和 Python 3.11；Windows 需要 .NET 9 和 Windows App SDK；Linux GTK 构建需要 GTK4/libadwaita/WebKitGTK/libsecret。
- 不要把 `.env`、邮箱凭据、真实邮件数据库、签名证书、商店密钥或生成安装包提交到 Git。

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

1. 协议、同步、存储和 HTML 净化问题优先在共享核心修复，并添加回归测试。
2. UI 保持平台原生；Preview 只能在显式预览模式使用。
3. 涉及 HTML/WebView、凭据、TLS、FFI/JNI、附件或发布脚本时，必须检查对应信任边界。
4. 更新用户可见字符串后运行 `python3 scripts/export-client-locales.py --check`。
5. 构建产物必须来自最终源码，记录版本、架构、SHA-256 和签名状态。
