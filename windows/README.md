# imyemail-cloud for Windows

Windows 10 1809+ / Windows 11 原生 WinUI 3 客户端，使用 MailKit、SQLite 和 WebView2。应用版本为 `0.2.1`，发布程序名为 `imyemail-cloud.exe`。

源码项目仍保留 `Chck.Mail.*` 目录和命名空间作为兼容标识；用户可见的产品、安装器、图标和包身份均为 `imyemail-cloud`。

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

开发安装器未签名；必须在对应 Windows 架构上验证安装、启动、升级、卸载、通知、高 DPI 和 WebView2。macOS 上的 .NET 测试不等于 WinUI 构建或发布验收。

凭据使用 Windows Credential Locker。HTML 阅读禁用脚本、下载、权限请求和非用户发起导航，外链只允许 HTTP(S)/mailto。
