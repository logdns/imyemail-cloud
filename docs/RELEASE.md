# 发布流程

## 版本

`imyemail-cloud-native` 使用 `native-v0.2.4`，`imyemail-cloud-mygo` 使用 `mygo-v0.3.3`；历史 `v0.2.0/v0.2.1/mygo-v0.3.0` 不覆盖、不改包名。原生版需同步 Rust、Apple、Android、Linux、Windows 及锁文件；MyGo 更新自身 Go/config 版本。Android versionCode 单调递增。身份与数据兼容规则见 [VERSIONS.md](VERSIONS.md)。

发布前先手动运行 `signing-preflight.yml`：在临时 macOS/Windows hosted runner 检查导入、无交互 trust、签名、固定指纹与清理；不会发布 fixture。不得把签名初始化挂起当作构建通过。Windows 仅在临时 runner 使用 LocalMachine 公共信任，私钥仍在 CurrentUser My，签名后全部删除；不修改用户机器的信任。

## 发布前检查

1. 工作树只包含预期源码和文档，不包含 `chckemail-old/`、凭据、数据库、签名材料或生成包。
2. 运行 [DEVELOPMENT.md](DEVELOPMENT.md) 中适用于当前主机的门禁，并确认 GitHub CI 全部通过。
3. 搜索旧品牌、收费、激活和网站代码残留；邮箱“应用密码/授权码”不属于软件许可证。
4. 确认所有客户端帮助/关于链接指向 `https://imy.email`，源码链接指向公开 GitHub 仓库。
5. 核对构建产物的版本、架构、内置核心、SHA-256 和签名状态。

## GitHub Release

`.github/workflows/release-build.yml` 支持手动输入 tag 和日期，重新测试并构建 macOS、iOS、Android、Linux、Windows 产物，最后创建可从 GitHub Releases 和 `/releases/latest` 发现的公开下载。原生版 tag 必须是 `native-vMAJOR.MINOR.PATCH`，并与源码版本一致。新增原生安装包统一前缀 `imyemail-cloud-native-`；MyGo 为 `imyemail-cloud-mygo-`。

发布后核验 Latest 对应标签与源码提交、附件清单和各独立 `.sha256`；下载后复算哈希。README 中带日期的附件直链必须固定到对应版本标签，不能使用 `/releases/latest/download/` 加旧日期文件名；Latest 页面可作为未来版本的动态入口。安装说明见 [INSTALL.zh-CN.md](INSTALL.zh-CN.md)。

当前手动发布采用稳定自签身份，见 [SIGNING.md](SIGNING.md)：

- macOS：证书自签 ZIP，未公证、非 Developer ID；
- iOS/iPadOS：arm64 unsigned IPA，包含设备版 Rust 核心，需自行签名和配置 provisioning profile；
- Android：固定密钥签名 APK，非 Play Store；
- Linux：DEB/tar.gz 的签名 SHA256SUMS，非 APT 仓库签名；
- Windows：项目程序及 Inno 安装器 Authenticode 自签，第三方运行库保留原签名，非默认公信。

仅手动 main 分支发布向临时 hosted runner 提供密钥；先验证 annotated tag 与 GITHUB_SHA 相等，拒绝现有 Release。原版设为 Latest，MyGo 仍独立预览。两版附公钥/证书、签名清单及源码/run provenance。下载后独立核验签名/哈希，不把自签描述为公证/公信/真机或商店验收。本地/PR 构建仍可 unsigned/ad-hoc。

## Homebrew tap

公开 Homebrew 源为 [logdns/homebrew-imyemail-cloud](https://github.com/logdns/homebrew-imyemail-cloud)，两个独立 cask 为 `imyemail-cloud-native` 与 `imyemail-cloud-mygo`。Homebrew 7 首次使用第三方 tap 时先 `brew tap logdns/imyemail-cloud`、`brew trust --tap logdns/imyemail-cloud`。旧 `imyemail-cloud` cask 保留为历史兼容入口，不切换到 MyGo。

两个 cask 独立固定版本 URL/哈希：`imyemail-cloud-native` 原生版 arm64，`imyemail-cloud-mygo` 桌面 amd64/arm64，均 macOS 14+。tap CI 检查安装、Bundle ID/版本/架构、固定证书代码签名、CLI 链接，再禁用 cleanup 卸载，不删除邮件或钥匙库。旧/新原生 cask 互斥，与 MyGo 可共存。

tap 每日分别读取 `native-v*` 与 `mygo-v*` Release，按各自版本更新；MyGo 明确包含预发布，不依赖 Latest。先固定证书指纹、验签清单并下载复算后才创建更新 PR。发布新版本时仍需人工确认该 PR 的 CI、版本、URL、哈希与签名边界；自动检测不能替代公证或真机启动验收。

## MyGo 桌面预览版

MyGo 使用 `.github/workflows/mygo-desktop.yml` 与 `mygo-vMAJOR.MINOR.PATCH`。六架构 CI、限定范围审查、产物复验后创建不可移动 annotated tag，main 手动 `release=true` 证书自签/发布；不替换旧版附件或原 cask，另用 MyGo token。能力见 [MYGO.zh-CN.md](MYGO.zh-CN.md)。

发布工作流会拒绝已存在的 Release，以及标签目标与工作流源码提交不一致的情况。测试对象必须是解压后的 ZIP/tar.gz，Linux 还要核对 DEB 内置核心与本次构建的 SHA-256 一致。发布后独立下载所有附件，核验 `SHA256SUMS`、架构、资源和内置核心，不使用旧包代替新代码。

## 回滚

不要覆盖既有 tag 或 Release。发现问题时撤下受影响附件、标记说明并发布递增补丁版本；保留源码提交和哈希以便审计。Homebrew 回滚通过恢复上一条已验证 cask 提交或发布递增修复版本完成，不得把同一版本静默改指向不同二进制。
