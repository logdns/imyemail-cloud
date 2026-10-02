# imyemail-cloud for Apple platforms

SwiftUI 原生客户端，支持 macOS、iOS 和 iPadOS；邮件协议通过同仓库 `core/` 的进程内接口执行。Bundle ID 为 `email.imy.cloud`，产品链接为 [https://imy.email](https://imy.email)。

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

凭据保存到系统 Keychain。邮件 HTML 在隔离 WebKit 视图中显示，禁用脚本、表单、自动外跳和默认远程资源加载。
