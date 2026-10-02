# 发布流程

## 版本

项目使用语义化版本。一次发布需同步更新 Rust、Apple、Android、Linux 和 Windows 的版本元数据；`0.2.0` 是首次 `imyemail-cloud` 开源版本。

## 发布前检查

1. 工作树只包含预期源码和文档，不包含 `chckemail-old/`、凭据、数据库、签名材料或生成包。
2. 运行 [DEVELOPMENT.md](DEVELOPMENT.md) 中适用于当前主机的门禁，并确认 GitHub CI 全部通过。
3. 搜索旧品牌、收费、激活和网站代码残留；邮箱“应用密码/授权码”不属于软件许可证。
4. 确认所有客户端帮助/关于链接指向 `https://imy.email`，源码链接指向公开 GitHub 仓库。
5. 核对构建产物的版本、架构、内置核心、SHA-256 和签名状态。

## GitHub Release

`.github/workflows/release-build.yml` 支持手动输入 tag 和日期，重新测试并构建 macOS、Android、Linux、Windows 产物，最后创建可从 GitHub Releases 和 `/releases/latest` 发现的公开下载。tag 必须是 `vMAJOR.MINOR.PATCH`，并与源码版本一致。

当前自动发布的是未签名开发产物：

- macOS：ad-hoc 签名 ZIP，未公证；
- Android：unsigned Release APK；
- Linux：未进行发行仓库签名的 DEB；
- Windows：未做 Authenticode 的 Inno Setup 安装器。

工作流将这些附件作为 GitHub Latest Release 提供下载，但标题和说明必须明确标记为未签名开发包。平台签名、公证、真机安装/升级/卸载和对应商店审核仍是独立的生产发布门禁，不能用 CI 开发包替代。

## 回滚

不要覆盖既有 tag 或 Release。发现问题时撤下受影响附件、标记说明并发布递增补丁版本；保留源码提交和哈希以便审计。
