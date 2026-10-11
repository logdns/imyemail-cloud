# 双版本开发与发布经验

整理日期：2026-10-11。来自 Native 0.2.4、MyGo 0.3.4 和双 cask 发布的实际结果；[原始证据与失败记录](../artifacts/20261011-self-signed-tracks/verification.md)保留历史事实。当前版本以 [VERSIONS.md](VERSIONS.md) 为准，本页维护可复用规则，不代替测试报告或完整安全审计。

## 1. 先确定工作区与版本归属

本仓库已是含 Git 的单体工作区，当前入口是 `core/`、`apple/`、`android/`、`linux/`、`windows/` 和 `desktop-mygo/`；不再按旧的同级 `chck-*` 目录或编号文档执行。读 `docs/README.md`、当前构建清单、工作树和远程地址，确认实际版本后再改动。另一个 `imyemail` Webmail/服务部署项目不是本仓库，不套用其 Pages、Docker/Operator 或 UI 模板发布流程。

| 改动 | 归属与回归 | 发布原则 |
| --- | --- | --- |
| Native UI/安装 | 对应平台及原生 CI | 递增 `native-v*`，保留原生应用/账号/升级身份 |
| MyGo UI/安装 | `desktop-mygo/`、Go/UI 夹具、六架构 CI | 递增 `mygo-v*`，独立 app、数据、cask，不宣称移动端或功能等价 |
| 共享协议/存储/凭据契约 | 相关 Rust crate 与两版调用路径；Windows 还核对 MailKit 行为 | 两版回归后按各自影响发布，不要求两版版本号相同 |
| 文档/skill 总结 | 路径、源码一致性、技能校验和引用检查 | 不触发签名、改标签、重打历史包或外部发布 |

skill 不提供提交、推送或信任修改的额外授权。操作范围仍由当次用户请求决定。

## 2. 改品牌，不批量改兼容身份

包名、app 显示名、下载入口使用 `imyemail-cloud-native` / `imyemail-cloud-mygo`。Native 的 Bundle ID、Android applicationId/JNI、URI scheme、OS vault、数据库路径、Windows Inno AppId/MSIX 身份和内部兼容可执行文件不跟随品牌全局替换。MyGo 的 `email.imy.cloud.mygo` 与配置目录独立，不自动迁移旧账号。

改名时先列出“显示名称”和“持久身份”两栏，再检查 `scripts/test-edition-names.py`。旧/新 Native 共享数据，不能同时运行；新 Native 与 MyGo 可以共存。Linux DEB 的 `Breaks/Replaces` 仅作用于版本受限的旧原生包，不能覆盖 MyGo。备份必须包含数据库和 OS 凭据库，不能只备份 SQLite；正常升级不用 `--zap`、purge 或 autoremove。

## 3. UI 参考不能改变安全与证据边界

Pulse 参考用于浅灰侧栏、蓝色选中态、圆角卡片和明暗层次；实现仍是自己的 Go/MyGo 组件、图标和邮件流程，不复制其品牌/图片或系统监控功能。MyGo 邮件保持原生纯文本、无 HTML/WebView/远程图片加载；统计注明是“已缓存/已加载”，不是服务端总量或实时同步指标。

测试不只断言布局字符串：覆盖明暗、880/1120 宽度、空态/筛选未命中、账号/文件夹切换、阅读/撰写动作和缓存统计含义。框架/API 依赖按 `go.mod`、`go.sum` 与工具链清单锁定，不依赖滚动上游代替当前源码验证。

README 展示真实 UI 的合成邮件截图，记录来源、源码、生成方式和哈希；Native Preview、MyGo headless renderer、真实窗口/GPU、真机与真实收发分别记录。截图不能提升验收等级，不能用设计稿冒充已实现软件。[截图规则](screenshots/README.md)是当前实例。

## 4. 隔离测试先验证环境，再诊断产品

本地安全/发布夹具使用无网络、非 root、只读源码/依赖、空环境白名单和专用 scratch；不挂载真实邮箱、宿主 OS vault 或环境中的凭据。资源边界包括 CPU、内存、PID、磁盘、单文件、墙钟，具体值按测试需要设置，不照搬一次运行。

本轮容易误判的环境问题：

- 空环境丢失 Cargo/SDK/clang/cache 路径。先列出所需工具，把明确路径加入白名单，不导出完整宿主环境。
- Docker `--ulimit fsize` 的值是字节，不是 shell `ulimit -f` 的常见块单位；限制过小会在复制夹具二进制时收到 `File size limit exceeded`。
- 需要执行模拟命令的 fixture 在 no-exec scratch 上报 126；仅给隔离 scratch 所需执行权限，不放宽网络、源码写权限或凭据访问。
- Keychain 交互测试不能靠改 HOME 获得无凭据隔离身份；系统钥匙库测试另用受控账户/runner。明确跳过项，不把跳过或超时算通过。

诊断记录保留第一次失败、确切错误、环境修正和同一测试的退出码。网络/真实凭据缺口不能通过降级为明文或关闭 TLS“解决”。

## 5. 先做签名预检，再付出全矩阵构建成本

发布签名只在显式请求、main-only、临时 hosted runner 执行。先让 macOS、Windows x64/ARM64 的 dummy 签名预检通过，确认固定发布者、签名、清理均可无交互完成。PR/开发构建不获取发布私钥，预检 fixture 不公开。

| 实际失败 | 可复用结论 |
| --- | --- |
| macOS 用户域 trust 交互挂起 | hosted 签名用私有临时 keychain 和当前脚本的非交互 admin-domain trust；不在维护者或用户机器上复制系统信任修改 |
| macOS 移除 trust 设置挂起 | 按固定指纹删除该系统证书与私有 keychain/P12；剩余 hosted trust-settings 由临时 runner 销毁，不视为本地机器已清理 |
| Windows CurrentUser Root 等待确认 | 当前 hosted-only 流程临时使用 LocalMachine Root，私钥仍在 CurrentUser My，finally 清理；用户安装不自动改系统信任 |
| ARM64 TrustedPublisher 写入拒绝 | 当前签名不需要该 store，不能用 x64 预检替代 ARM64，也不能通过扩大权限盲目重试 |
| 提取 macOS 签名证书前缀被识别为目标 | 使用 `codesign -d --extract-certificates=PREFIX APP`，参数的 `=` 不省略 |

固定身份以 [SIGNING.md](SIGNING.md) 和 `signing/` 公共材料为准，不把临时证书当长期发布者。先签二进制/安装包，再算最终包哈希，最后签清单；第三方运行库保留原签名。自签、公证、默认系统信任、商店审批和真机验收是不同层次。iOS unsigned IPA 仍需要 Apple provisioning。

## 6. CI、标签、Release 是不同状态

发布前完成源码、版本和必要文档，等待指定 SHA 的对应回归完成，然后建立 annotated tag；发布 job 检查 tag 与运行源码相等，拒绝既有 Release。不要在启用同一 concurrency group 的 `cancel-in-progress` 发布期间推送会触发新运行的提交，避免取消正在发布的矩阵。

失败候选标签不是可下载 Release。签名或构建失败后保留失败源码/日志，修复用递增补丁，不移动标签或替换历史包。文档只链接真实已发布 Release，不把失败候选、源码 tag 或部分成功作业写成完整下载入口。

读取失败步骤后再决定重跑：明确的基础设施/超时波动可定向重跑失败 job 并保留证据；同样故障再现时进入修复/阻塞分析，不无限重跑。用 `gh run watch --interval 60 --exit-status` 等待，必要时取已完成 job 日志，不每几秒请求整轮状态。

发布完成后可做不改变软件的证据收口；确为纯文档/证据提交时可根据维护策略使用 `[skip ci]`，明确没有新增软件 CI，不能用于源码、脚本、签名、依赖或 workflow 改动。总结文档不会自动取得 commit/push 权限。

## 7. 下载复验是发布门禁，不是额外美化

从最终 Release 独立下载，固定完整证书指纹，验证清单签名，再校验精确文件名/包哈希。检查版本、CPU、资源、核心 helper、依赖许可证、源码 provenance；MyGo 主程序/helper 必须匹配同一构建，DEB/tar 的 helper 也要一致。下载的代码/解析测试在隔离环境运行，不向维护者数据启动安装器。

区分独立下载静态检查与 hosted runner 的执行验签：检查 Windows PE 签名表不等于独立安装或系统信任验收。验证所有 README 包链接和匿名清单/截图可获取；HEAD 200 是可达性证据，不能代替完整下载验签。GitHub Latest 归 Native，MyGo 用明确 tag。旧版保留 tag/target 与资产 ID、名称、大小、digest 快照；摘要一致不代替需要时的历史包逐字节比较。

## 8. Homebrew 是第二条独立发布链

tap 维护两个固定版本/URL/哈希的 cask，MyGo 包含自己的 prerelease 选择，不能直接沿用 Native Latest。同步器先固定证书、验签清单、复算真实包哈希，再更新定义和双语版本表，创建可审核 PR，不自动合并。

项目已验证的定义格式：MyGo 使用 `sha256 arm: ..., intel: ...`，Native/legacy cask 的 `conflicts_with` 在 `depends_on` 前。修复定义必须同时修复生成器，防止每日同步恢复错误；以当前 Homebrew style/audit 为准，不把旧格式永久写死。

bot 产生的 PR 不保证自动触发 push/PR workflows，当前同步显式 dispatch 分支检查。CI 用临时 macOS 机器做 Native arm64、MyGo arm64/Intel 的 style/audit、短 token 安装、Bundle ID/版本/CPU/固定签名/CLI、卸载；在双方支持的 arm64 上同时验证并存和移除 Native 不移除 MyGo，不要求不存在的 Native Intel 包。首次 tap trust 是 Homebrew 仓库授权，不等于 OS 对自签 app 的信任。

不在维护者真实 Homebrew 环境做破坏性生命周期验收；本轮改用临时 runner，并设置 `HOMEBREW_NO_INSTALL_CLEANUP=1` 避免影响无关依赖。不用可能等待 UI/引擎初始化的 `--help` 充当安装成功检查。升级仍需人工审查 PR，安装通过不能替代 GPU 启动、长期账户恢复或 Gatekeeper acceptance。

## 9. 文档、证据与 skill 各司其职

- `docs/DEVELOPMENT.md`：日常入口和门禁；本页：经验与决策；`docs/RELEASE.md`：发布 checklist；`docs/SIGNING.md`：准确签名命令与信任边界。
- `docs/VERSIONS.md`：当前两版/兼容/历史候选；根双语 README 与安装文档：用户选择；截图来源文件：图像 provenance。
- `artifacts/YYYYMMDD-topic/verification.md`：日期、源码、命令/退出码、成功和失败、产物哈希/签名、审查范围与缺口；历史不改写成最新结果。
- 本地 `chck-client-development` / `chck-security-audit`：稳定路由和重要防错规则；不包含凭据、固定一次性 PID/临时目录，不作为公开安装产物。

文档检查关注失效路径、实际存在的 Release、源码/版本/截图对应关系和相对链接；skill 校验只证明结构，另用“只改文档”“共享核心修复”“失败候选重发”“双 cask 同步”场景检查行为。普通文档总结不启动完整审计或发布任务。

安全审查者检查源码/隔离夹具和已有 runner 证据，缺失事实列为待验证；hosted 签名、dispatch 和安装生命周期由另行获准的开发/发布流程执行。聚焦问答不自动创建完整审计目录或报告。回滚默认保留历史资产并标记问题，撤下附件是需明确授权的安全处置，不是普通修复动作。

## 下一阶段验收

按需求安排专门测试，不能从本轮 CI 自动升级状态：真实邮箱收发/取消恢复；MyGo 真窗口/GPU/输入法/高 DPI；各平台 OS vault ACL、锁定和跨版本签名访问；携带数据库及凭据的长期升级/回滚/重启恢复；Android 真机 JNI 与页大小；Apple provisioning、公证/商店及 Windows 默认信任。完整协议、上游框架与资源生命周期审计另列范围，本轮限定范围检查不代表“无漏洞”。
