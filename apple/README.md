# imyemail-cloud-native for Apple platforms

SwiftUI 原生客户端，支持 macOS、iOS 和 iPadOS；邮件协议通过同仓库 `core/` 的进程内接口执行。Bundle ID 为 `email.imy.cloud`，产品链接为 [https://imy.email](https://imy.email)。

原版 `0.2.4` 与 MyGo `0.3.4` 独立保留：[安装/升级/卸载](../docs/INSTALL.zh-CN.md) · [自签校验](../docs/SIGNING.md) · [历史版本恢复](../docs/VERSIONS.md)。MyGo 仅桌面，另用 `brew install imyemail-cloud-mygo`。

## Homebrew 安装（Apple Silicon）

```bash
brew tap logdns/imyemail-cloud
brew trust --tap logdns/imyemail-cloud
brew install imyemail-cloud-native
```

Homebrew 7 对第三方 tap 要求一次显式信任。`brew upgrade imyemail-cloud-native` 更新原生版；[公开 tap](https://github.com/logdns/homebrew-imyemail-cloud) 不切换到 MyGo。发布包固定证书自签、未 Apple 公证，首次启动受 Gatekeeper 检查；本地默认仍是 ad-hoc。

### macOS 首次打开被拦截

开发者目前没有 Apple Developer 账号，未使用 Developer ID 签名或 Apple 公证；当前包为自签，不是完全未签名。先验签、核对官方包哈希，再按[两种解决方法](../docs/INSTALL.zh-CN.md)处理：优先 **系统设置 → 隐私与安全性 → 仍要打开**；若已确认可信且仅下载隔离导致拦截，可打开终端执行 `xattr -dr com.apple.quarantine "/Applications/imyemail-cloud-native.app"`，然后重开应用。该命令降低指定 app 的隔离保护，不能修复真实损坏/签名异常，也不能用于绕过恶意软件告警；不关闭全局 Gatekeeper/SIP。

## 本地验证

```bash
swift test -Xswiftc -warnings-as-errors
swift build
./scripts/build-native.sh
./scripts/package-app.sh
```

macOS 开发包输出到 `dist/imyemail-cloud-native.app` 及 ZIP。默认使用 ad-hoc 签名，不代表 Developer ID、公证或商店发布。

iOS/iPadOS 模拟器：

```bash
brew install xcodegen
./scripts/package-ios-simulator.sh
# 或运行完整模拟器验证
./scripts/verify-ios.sh
```

模拟器 `.app` 不是 IPA；真机、后台通知、provisioning、Archive 和 TestFlight 需要独立验收。只有显式设置 `IMYEMAIL_CLOUD_PREVIEW=1` 才使用样例数据。

iOS/iPadOS arm64 未签名 IPA：

```bash
rustup target add aarch64-apple-ios
brew install xcodegen
./scripts/package-ios-device.sh
```

该脚本输出标准 `Payload/imyemail-cloud-native.app` 结构的 IPA，并验证设备架构、iOS 平台标记、Bundle ID、内置 Rust C ABI 和无签名状态。产物仅供自行签名或后续 Apple 发布流水线使用；没有有效 Apple 证书和 provisioning profile 时不能承诺直接安装、TestFlight 或 App Store 分发。

凭据保存到系统 Keychain。邮件 HTML 在隔离 WebKit 视图中显示，禁用脚本、表单、自动外跳和默认远程资源加载。
