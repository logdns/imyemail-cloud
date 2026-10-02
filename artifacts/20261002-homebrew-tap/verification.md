# imyemail-cloud Homebrew tap 发布验证

## 范围与边界

- 日期：2026-10-02，Asia/Shanghai。
- 主仓库：`logdns/imyemail-cloud`；Homebrew tap：`logdns/homebrew-imyemail-cloud`。
- 目标：公开名为 `imyemail-cloud` 的 Homebrew cask；首次 tap 和信任后支持用户要求的 `brew install imyemail-cloud`，同时安装 macOS 应用和同名 CLI 链接。
- 平台：Apple Silicon arm64、macOS 14+；Intel 不在当前发布范围。
- 安全审查：针对 tap 源、固定下载 URL、SHA-256、架构、签名、自动更新、安装/卸载边界的聚焦 source-first 检查，不是完整安全审计。
- 签名边界：当前 GitHub Release ZIP 采用 ad-hoc 签名但未 Apple 公证；Homebrew 哈希校验不等于 Developer ID、公证或 App Store 验收，首次 GUI 启动仍受 Gatekeeper 检查。

## 实现

- cask token：`imyemail-cloud`；固定版本 `0.2.1` 和 GitHub Release macOS ZIP SHA-256 `91f6ac78b0d0571e2595982f3058764c59463f68628f1e1170a3267d35cf2ed2`。
- 安装 `imyemail-cloud.app` 到 `/Applications`，并链接应用包内 CLI 到 Homebrew `bin`。
- 正常卸载只移除 Homebrew 管理的应用与 CLI 链接，不声明或执行邮箱数据库、偏好设置、缓存、日志或 Keychain 凭据清理。
- tap CI 执行 style、strict audit、livecheck、短命令安装、Bundle ID/版本/架构/ad-hoc 签名/CLI 链接验证及卸载。
- 每日同步 workflow 从 GitHub Latest Release 解析唯一 macOS ZIP 和 `.sha256`，复算后更新版本、URL 和 SHA-256，并创建待 CI 审核的 PR。
- 中英文主 README、Apple README 和发布文档加入安装、更新、卸载与签名边界。

## 本地验证

- Homebrew `7.0.7`；`brew style --cask` 通过；`brew audit --cask --strict` 通过；`brew livecheck --cask` 返回 `0.2.1 ==> 0.2.1`。
- Homebrew 7 首次拒绝加载未信任的第三方 tap，执行 `brew trust --tap logdns/imyemail-cloud` 后，实际运行 `brew install imyemail-cloud` 成功。
- 安装结果：`/Applications/imyemail-cloud.app`；Bundle ID `email.imy.cloud`；版本 `0.2.1`；主程序 Mach-O arm64；`codesign --verify --deep --strict` 通过；签名类型为 ad-hoc。
- CLI 链接：`/opt/homebrew/bin/imyemail-cloud -> /Applications/imyemail-cloud.app/Contents/MacOS/imyemail-cloud`。
- `spctl --assess` 按预期拒绝未公证开发包；文档和 cask caveat 已明确该边界，没有把哈希或 ad-hoc 签名描述成 Apple 公证。
- 安装态运行 CLI `--help` 出现等待，已终止测试进程并将 CI 限定为不执行该命令的静态链接/包验证；这是未验收项，不在本轮修改客户端行为。
- 本地卸载验证触发 Homebrew 7 默认 `autoremove`，意外移除 4 个与项目无关的依赖公式；已恢复 `appstream`、`libfyaml`、`libxmlb`、`openjdk`，后续 CI 设置 `HOMEBREW_NO_INSTALL_CLEANUP=1`，避免扩大清理范围。

## 远端发布

- 公开仓库：[logdns/homebrew-imyemail-cloud](https://github.com/logdns/homebrew-imyemail-cloud)，默认分支 `main`，最终 tap 提交 `24fe15bf3de2cc73476146f37e94282096e75628`。
- tap 最终 CI：[run 36985809527](https://github.com/logdns/homebrew-imyemail-cloud/actions/runs/36985809527)，cask style、strict audit、livecheck、短命令安装、包验证和卸载全部成功。
- Latest Release 同步 workflow 手动复验：[run 36985813218](https://github.com/logdns/homebrew-imyemail-cloud/actions/runs/36985813218)，识别当前 `v0.2.1`、远端 macOS ZIP 和 `.sha256` 一致，无需创建更新 PR。
- 独立远端安装复验：先 untap 本地工作副本，再从公开 GitHub 仓库重新执行 tap、trust、`brew install imyemail-cloud`；远端 cask `0.2.1` 安装成功，随后在禁用 Homebrew cleanup 的条件下卸载成功。
- [v0.2.1 GitHub Release](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.1) 说明已增加 Homebrew 首次配置、短命令安装/更新、公开 tap 和未公证边界；Release 标签及 15 个附件未移动或替换。
- 主仓库文档 CI 待推送后记录；不为纯文档和独立 tap 发布移动 `v0.2.1` 标签或重建客户端包。

## 未验收项

- 未进行 Developer ID 签名、Apple 公证、Intel 构建、真实邮箱收发、GUI 首次启动授权或长期升级/回滚数据兼容性测试。
- Homebrew 官方 `homebrew-cask` 尚未收录；因此每台机器首次需要 tap 和 Homebrew 7 的 trust，一次配置完成后才使用短命令。
