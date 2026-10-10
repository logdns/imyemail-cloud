# 架构

[双版本安装](INSTALL.zh-CN.md) · [签名与发布信任边界](SIGNING.md) · [历史源码保留](VERSIONS.md)。两版 App ID/数据独立，原版不被 MyGo 替代。

imyemail-cloud 采用共享邮件核心加原生 UI 的结构：

```text
Apple / Android / Linux / Windows
              │
       native adapter / FFI
              │
     Rust core: protocol + sync
              │
   IMAP / SMTP / local SQLite
```

## 组件边界

- `core/` 是协议、MIME、同步、存储、服务商预设和净化策略的权威实现。
- Apple 和 Android 通过进程内 FFI/JNI 使用核心；Linux 直接链接 Rust crates；Windows 使用保持相同行为契约的 MailKit 适配层。
- 平台凭据进入 Keychain、Android Keystore、Windows Credential Locker 或 Linux libsecret。SQLite 不保存邮箱密码。
- 客户端显示的 HTML 必须先净化；脚本、表单、对象、自动导航、下载和默认远程资源加载均不可信。
- `push-bridge/` 只接收经 HMAC 验证的最小元数据，不传递主题、发件人或正文。
- `email-testkit/` 仅用于隔离测试，夹具账号不能用于公网服务。

## 开源边界

仓库没有网站/Web 应用、支付、订阅、软件许可证、激活、设备名额或商业授权服务。客户端的全部本地功能不受软件许可证门禁控制。邮箱服务商的密码、OAuth token、应用密码或邮箱授权码仍是正常登录凭据。

## 兼容名称

`desktop-mygo/` 是独立的 Go/MyGo 原生 UI 桌面重构版，通过私有 stdin/stdout 调用同次构建的 Rust CLI 核心。它使用独立数据目录和系统凭据服务，不迁移旧客户端状态，不启用 Web UI、监听端口或不签名的自动更新。当前能力边界见 [MYGO.zh-CN.md](MYGO.zh-CN.md)。

部分 crate、Swift 类型、Kotlin 源目录和 .NET 命名空间仍使用 `chck`/`Chck`，用于保持 ABI、数据库和工程兼容。它们不是对外品牌。仓库品牌为 `imyemail-cloud`，产品分别为 `imyemail-cloud-native` 与 `imyemail-cloud-mygo`，产品链接保持 `https://imy.email`。原生内部可执行文件、应用/钥匙库/数据库身份保持旧值；MyGo 使用独立身份。包名、标签、数据和升级矩阵见 [VERSIONS.md](VERSIONS.md)。
