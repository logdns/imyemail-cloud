# 开发经验与 skill 整理验证

日期：2026-10-11，Asia/Shanghai。基础源码/文档提交 `9b2bc4d528db954ce7dc3a27feef12b66f3f9975`。本轮是文档与本地 skill 维护，不改软件、workflow、签名脚本、依赖、版本、tag、Release、历史安装包、tap 或本机信任；没有新软件构建、真机验收或安全审计。

## 整理范围

- 新增 [DEVELOPMENT-LESSONS.md](../../docs/DEVELOPMENT-LESSONS.md)：从原始双版本验证记录提炼工作区识别、改名兼容、Pulse 原生 UI、隔离环境诊断、签名预检、CI 并发、不可移动标签、下载验签、独立 tap 与待验收项。
- 更新开发指南、发布 SOP 和文档索引，纠正“禁止签名证书入 Git”中的歧义：私钥/P12 禁入，固定公开证书/公钥是允许审查的验签输入。
- 本地 `chck-client-development`：更新 Native/MyGo 自动发现说明、UI 元数据、当前单体/历史工作区路由、平台参考，新增双版本发布参考。去掉旧“无 Git”、同级路径/已移除文档是当前必读项的错误假设。
- 本地 `chck-security-audit`：更新当前源码、MyGo stdio/vault、签名/tap 与证据分级；保留历史服务的条件路由，明确当前没有 Web/支付/商业授权，不把另一个 imyemail Webmail 项目引入客户端任务。
- skill 名称和默认自动发现策略保持；skill 仅在维护者本地，不复制到公开软件源码。另一个项目的 `imyemail-development-release` 未修改。

## 验证

使用 skill-creator 的 `quick_validate.py` 校验两个 skill，均退出 0，`Skill is valid!`。首次运行缺少 PyYAML；只在独立临时 venv 安装 `PyYAML==6.0.3` 后重验，不修改系统 Python 或 Homebrew。校验涉及维护工具和文档，没有执行目标客户端代码。

相对引用检查：25 份活跃文档、156 个本地链接均有实际目标；两个 skill 的 6 个 Markdown reference 链接均可解析。UI 元数据 YAML 可读，short_description 长度符合 25–64 字符，已有 default_prompt 引用准确 skill 名称，默认自动发现未禁用。`git diff --check` 退出 0。结构/引用校验不证明运行态行为或完整 skill 质量。

独立只读 forward-test 覆盖普通文档总结、共享 UIDVALIDITY 修复、ARM64 失败候选重发、双 cask 生命周期四个场景。评估未执行客户端代码、网络、写入或发布。前三个场景路由符合任务边界；第四个指出架构措辞歧义。另发现审查输出/权限与回滚规则需澄清，均已做有依据的最小修正：

| 独立复核发现 | 修正 |
| --- | --- |
| 审查 skill 无条件要求外部完整审计输出目录 | 限定为完整审计；普通 guidance/聚焦检查不自动建报告，按请求返回或保留项目证据 |
| hosted 签名预检可能被理解为审查者自行 dispatch 权限 | 明确审查者检查源码/夹具/现有证据，缺口报告；另行授权的开发/发布流程执行 hosted 操作 |
| “两架构安装”可能要求并不存在的 Native Intel 包 | 明确 Native arm64、MyGo arm64/Intel，双方共存测试仅 arm64 |
| 默认“撤下受影响附件”违背旧包保留意图 | 默认保留历史资产并标记问题、递增修复；撤下仅明确授权的安全处置 |

修正后再次通过两个 skill 的结构、UI 元数据、引用和仓库 diff 检查；没有用措辞断言测试替代独立场景复核。完整软件/真实设备结果仍引用前轮证据，不提升其验收等级。

## 保留与边界

原始日期证据和 `chckemail-old/` 保持；临时校验脚本/venv 不进入 Git，不含真实邮箱、私钥或环境转储。此轮不 commit/push/dispatch，也不重跑此前已通过的整个平台矩阵；后续若要求发布文档，再单独处理授权和远端状态。
