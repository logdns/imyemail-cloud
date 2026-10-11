# 开发经验与 macOS 提醒文档发布

日期：2026-10-11，Asia/Shanghai。用户在前两轮本地整理后明确要求“发布更新文档”。本轮授权范围为文档提交/推送及两版 Release 说明同步；软件版本、签名、标签目标、下载附件、tap cask 和宿主安全设置不变。

## 主仓库

公开仓库 `logdns/imyemail-cloud`，main 文档提交 `b1990384d5cb19ba8a036180954205e863e651d2`：根双语 README、Apple README、开发经验/指南、双语安装指南、发布/签名指南、索引及前两轮记录共 13 个 Markdown 文件。提交前检查 staged 清单仅包含 `.md`，没有软件/workflow/密钥/归档。提交和 `git push origin main` 成功。

本地两个 skill 已在前轮更新与校验，留在维护者 skill 目录，不公开其本地操作配置。前轮报告的“未提交/推送”描述的是当时的整理状态，不是本次发布状态。

## Release 说明

- [Native 0.2.4](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.4)
- [MyGo 0.3.4](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.4)

两版仅更新 body：新增未使用 Apple Developer ID/未公证、当前证书自签而非完全未签名、首次打开提示、系统设置推荐路径、按版本限定的终端命令与验签/风险提醒；保留原下载、截图、源码与历史说明。回读 JSON body 与准备文件逐字节一致。首次使用 CLI `--jq` 对比多了输出换行；改用 `jq -jr '.body'` 比较真实 body 后通过，不因此重复发布或改写资产。

编辑前后 tag/target 与每个附件的 ID、名称、大小、digest 完全一致；没有上传、删除、替换或重新签名包。Latest 仍是 `native-v0.2.4`，MyGo 仍独立预览版。Homebrew 的安装说明链接指向 main 的新指南，不修改 cask/哈希或触发 tap 发布。

## 检查与公网复验

- 25 份活跃文档的 164 个本地链接可解析；两个本地 skill 的 6 个引用和 UI 元数据有效。
- `git diff --check`、staged diff 检查和两条 `xattr` 示例的 `bash -n` 语法检查退出 0；没有执行隔离属性修改命令。
- 匿名 HTTPS 下载公开 `README.md`、`README.zh-CN.md`、双语 INSTALL 和 DEVELOPMENT-LESSONS，均与提交后的本地内容逐字节相同。
- 推送后 fetch 确认 HEAD 与 origin/main 相同，工作树干净；本记录随后以单独文档证据提交收口，不改变软件来源。

文档及本记录提交使用 `[skip ci]`，没有调度软件全矩阵、签名预检、构建打包或真实设备验收，也不把前轮软件 CI 写成本轮新测试。终端方法不能修复实际损坏/恶意软件或提供公证，不保证具体 macOS 版本必定允许打开。公开软件测试与产物边界仍以原双版本验证记录为准。
