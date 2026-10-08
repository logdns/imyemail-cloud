# 2026-10-08 文档与发布入口整理

- 范围：主仓库双语安装指南、README 下载直链、发布流程、CI 文档门禁和独立 Homebrew tap 的双语 README；未修改客户端或重建二进制。
- 主仓库提交：`1190ecb8f67b6a8b865e2b5dc49d9d3b860f399b`；tap 提交：`d4903cedd20b5a48ef7a28bb2899997945c401ff`。旧版 `chckemail-old/` 未修改，未加入公开历史。
- 本地核验：`git diff --check`、`python3 scripts/export-client-locales.py --check`（643 条 × 6 种语言）、Ruby 解析两个 GitHub Actions YAML 均通过；对照 GitHub 最新 Release API 核对中英文 README 中全部 `v0.2.1` 直链的附件名均存在。
- 公开发布：[v0.2.1 Latest Release](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.1) 保持正式发布状态，共 15 个附件，未移动标签、替换二进制或更改哈希；仅在说明中增加双语指南入口。旧发布的完整附件哈希与 IPA 结构验证见 [iOS 发布验证](../20261002-ios-ipa-release/verification.md)。
- Homebrew cask 仍从固定版本的 GitHub macOS ZIP 安装，SHA-256 不变；tap README 不再建议执行已知可能阻塞的 CLI `--help`。安装/卸载、架构、签名和供应链检查见 [Homebrew 发布验证](../20261002-homebrew-tap/verification.md)。
- 聚焦安全复核：只改文档和发布说明；检查动态 Latest 与固定日期附件误匹配的风险，改用标签固定直链；保留未公证、未签名和自行签名边界。未进行全面安全审计、真机启动或本轮二进制重建。
- CI：[主仓库本轮检查](https://github.com/logdns/imyemail-cloud/actions/runs/37713836600)、[tap 本轮检查](https://github.com/logdns/homebrew-imyemail-cloud/actions/runs/37713838735)；运行结果以对应 GitHub Actions 页面为准。
