# 发布流程

## 版本

项目使用语义化版本。一次发布需同步更新 Rust、Apple、Android、Linux 和 Windows 的版本元数据；`0.2.0` 是首次 `imyemail-cloud` 开源版本，`0.2.1` 补充 iOS/iPadOS arm64 unsigned IPA。

## 发布前检查

1. 工作树只包含预期源码和文档，不包含 `chckemail-old/`、凭据、数据库、签名材料或生成包。
2. 运行 [DEVELOPMENT.md](DEVELOPMENT.md) 中适用于当前主机的门禁，并确认 GitHub CI 全部通过。
3. 搜索旧品牌、收费、激活和网站代码残留；邮箱“应用密码/授权码”不属于软件许可证。
4. 确认所有客户端帮助/关于链接指向 `https://imy.email`，源码链接指向公开 GitHub 仓库。
5. 核对构建产物的版本、架构、内置核心、SHA-256 和签名状态。

## GitHub Release

`.github/workflows/release-build.yml` 支持手动输入 tag 和日期，重新测试并构建 macOS、iOS、Android、Linux、Windows 产物，最后创建可从 GitHub Releases 和 `/releases/latest` 发现的公开下载。tag 必须是 `vMAJOR.MINOR.PATCH`，并与源码版本一致。

发布后核验 Latest 对应标签与源码提交、附件清单和各独立 `.sha256`；下载后复算哈希。README 中带日期的附件直链必须固定到对应版本标签，不能使用 `/releases/latest/download/` 加旧日期文件名；Latest 页面可作为未来版本的动态入口。安装说明见 [INSTALL.zh-CN.md](INSTALL.zh-CN.md)。

当前自动发布的是未签名开发产物：

- macOS：ad-hoc 签名 ZIP，未公证；
- iOS/iPadOS：arm64 unsigned IPA，包含设备版 Rust 核心，需自行签名和配置 provisioning profile；
- Android：unsigned Release APK；
- Linux：未进行发行仓库签名的 DEB；
- Windows：未做 Authenticode 的 Inno Setup 安装器。

工作流将这些附件作为 GitHub Latest Release 提供下载，但标题和说明必须明确标记为未签名开发包。平台签名、公证、真机安装/升级/卸载和对应商店审核仍是独立的生产发布门禁，不能用 CI 开发包替代。

## Homebrew tap

公开 Homebrew 源为 [logdns/homebrew-imyemail-cloud](https://github.com/logdns/homebrew-imyemail-cloud)，cask token 为 `imyemail-cloud`。Homebrew 7 首次使用第三方 tap 时必须依次执行 `brew tap logdns/imyemail-cloud`、`brew trust --tap logdns/imyemail-cloud`，之后 `brew install imyemail-cloud` 和 `brew upgrade imyemail-cloud` 使用短名称即可。

tap 的 cask 必须固定 GitHub Release 版本 URL 和 macOS ZIP 的 SHA-256，只支持当前发布包实际具备的 Apple Silicon 与 macOS 14+。tap CI 需要实际执行短命令安装，核对 Bundle ID、版本、arm64、ad-hoc 签名和 CLI 链接，再卸载且不删除用户邮件数据或 Keychain 凭据。

tap 每日读取 GitHub Latest Release，要求恰好存在一个符合命名规则的 macOS ZIP 及其独立 `.sha256`，下载并复算成功后才创建 cask 更新 PR。发布新版本时仍需人工确认该 PR 的 CI、版本、URL、哈希与签名边界；自动检测不能替代公证或真机启动验收。

## 回滚

不要覆盖既有 tag 或 Release。发现问题时撤下受影响附件、标记说明并发布递增补丁版本；保留源码提交和哈希以便审计。Homebrew 回滚通过恢复上一条已验证 cask 提交或发布递增修复版本完成，不得把同一版本静默改指向不同二进制。
