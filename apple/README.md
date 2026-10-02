# imyemail-cloud for Apple platforms

SwiftUI 原生客户端，支持 macOS、iOS 和 iPadOS；邮件协议通过同仓库 `core/` 的进程内接口执行。Bundle ID 为 `email.imy.cloud`，产品链接为 [https://imy.email](https://imy.email)。

## Homebrew 安装（Apple Silicon）

```bash
brew tap logdns/imyemail-cloud
brew trust --tap logdns/imyemail-cloud
brew install imyemail-cloud
```

Homebrew 7 对第三方 tap 要求一次显式信任。之后可以使用 `brew upgrade imyemail-cloud` 更新。公开 cask 源码在 [logdns/homebrew-imyemail-cloud](https://github.com/logdns/homebrew-imyemail-cloud)；当前包采用 ad-hoc 签名且未经过 Apple 公证，首次启动仍受 macOS Gatekeeper 检查。

## 本地验证

```bash
swift test -Xswiftc -warnings-as-errors
swift build
./scripts/build-native.sh
./scripts/package-app.sh
```

macOS 开发包输出到 `dist/imyemail-cloud.app` 及 ZIP。默认使用 ad-hoc 签名，不代表 Developer ID、公证或商店发布。

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

该脚本输出标准 `Payload/imyemail-cloud.app` 结构的 IPA，并验证设备架构、iOS 平台标记、Bundle ID、内置 Rust C ABI 和无签名状态。产物仅供自行签名或后续 Apple 发布流水线使用；没有有效 Apple 证书和 provisioning profile 时不能承诺直接安装、TestFlight 或 App Store 分发。

凭据保存到系统 Keychain。邮件 HTML 在隔离 WebKit 视图中显示，禁用脚本、表单、自动外跳和默认远程资源加载。
