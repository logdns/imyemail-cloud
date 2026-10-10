# imyemail-cloud-native for Windows

Windows 10 1809+ / Windows 11 原生 WinUI 3 客户端，使用 MailKit、SQLite 和 WebView2。原版 `0.2.2`，发布程序名 `imyemail-cloud.exe`；MyGo `0.3.1` 独立便携程序，不替换原版。[安装/更新/卸载](../docs/INSTALL.zh-CN.md) · [固定证书 Authenticode 自签校验](../docs/SIGNING.md) · [历史版本](../docs/VERSIONS.md)。

源码项目仍保留 `Chck.Mail.*` 目录和命名空间作为兼容标识；用户可见产品与安装包名为 `imyemail-cloud-native`。程序/图标文件名、MSIX 身份和 Inno AppId 保留旧值，保证原生版升级连续性，不是第三个版本。

## 可移植测试

macOS/Linux 可以运行不依赖 WinUI 的核心测试：

```bash
dotnet test Chck.Mail.Tests/Chck.Mail.Tests.csproj -c Release
```

## Windows 验证和安装器

在 Windows PowerShell 中：

```powershell
.\scripts\verify-windows.ps1
# 或仅验证一种架构
.\scripts\verify-windows.ps1 -Architectures ARM64
```

脚本构建 x64/ARM64 自包含目录、运行测试并记录哈希。安装器入口：

```powershell
.\scripts\build-installer.ps1 -Architecture x64 `
  -PublishDirectory .\artifacts\windows-build\win-x64
```

本地开发安装器默认未签名；手动 CI 发布会对本项目程序及安装器做固定证书自签（非公信/SmartScreen 信誉）。仍需对应架构设备验收安装、启动、升级、卸载、通知、高 DPI 和 WebView2；macOS .NET 测试不等于此验收。

凭据使用 Windows Credential Locker。HTML 阅读禁用脚本、下载、权限请求和非用户发起导航，外链只允许 HTTP(S)/mailto。
