# MyGo 桌面重构与发布验证

## 范围

- 日期：2026-10-10，Asia/Shanghai。主仓库 `logdns/imyemail-cloud`。
- 新增 `desktop-mygo/`：Go 1.27.1 原生 UI，MyGo 固定提交 `b8beccc577daa00fed7c24112b0a7a0550d4825b`；桌面版本 `0.3.0`，独立应用 ID、数据目录和预览发布标签。
- 原生桌面账号→文件夹同步→缓存收件箱→纯文本阅读→草稿/发送→outbox 状态路径。旧客户端、移动端、Homebrew cask、`v0.2.1` 下载和 `chckemail-old/` 保留。
- 无官网/Web UI，不新增收费激活，不监听本地 HTTP。密码经私有 stdin/stdout 进入 Rust 核心；严格 OS 凭据后端不使用旧明文 sidecar。

## 初步本地验证

- 使用独立 Colima `imyemail-mygo`，不改变默认 Docker context。审计容器网络 `none`，根文件系统、源代码和依赖只读，环境由 `env -i` 白名单建立；用户 65534，禁用 capabilities 和权限提升。
- 容器限制：内存/交换总额 4 GiB、2 CPU、256 PID、1024 文件句柄、1 GiB 单文件、6 GiB scratch tmpfs、900 秒 CPU、1200 秒墙钟。未挂载宿主 HOME、钥匙串、Docker socket、GitHub 凭据或真实邮箱。
- 前一源码快照：Go vet 与 9 项 Go 测试（race、真实进程管道的正常/错序/超时子测试、UI 阅读与错误/窄窗口）通过；Rust desktop 5 项 IPC/请求限额/错误脱敏/钥匙库失败不写明文测试通过；desktop-vault Clippy 通过。
- 从该快照在禁外网容器构建 Linux arm64 Release 核心与 MyGo 原生应用，生成 DEB/tar.gz、依赖许可证和 SHA-256；打包后 `--version`、`--self-test` 验证头部版本与实际核心通信通过。
- 后续发送账号绑定与版本一致性改动须以最终源码 CI 再次验证；不能用前一快照替代最终发布门禁。

## 审计与发布门禁

- 安全审计限定为新增桌面版及其 IPC、凭据适配、打包/发布工作流。旧协议/移动端不属于完整覆盖；没有兼容的旧 coverage ledger。
- 修正发送状态误报：不把返回 draft ID 当作已送达，不主动 flush 其他已排队消息；发送结果未知时提示先检查 outbox，不盲目重投。编辑中的发送账号固定，避免侧栏账号切换改变待发送动作。
- macOS 系统调用不能在本机满足完整硬资源限制，未在宿主账户执行审计目标；跨平台打包/包自检交给平台 CI，不声称真机或真实邮箱验收。
- 正式创建下载前必须等待新 MyGo 六架构 CI、原工程回归与限定范围审计；发布标签不可移动，下载附件必须独立复算 SHA-256。

## 未验收边界

- Developer ID/Apple 公证、Authenticode、真实邮箱收发、GUI 真机/长期升级/回滚数据兼容性未验收。
- 新 UI 暂为英文；OAuth、附件、HTML 富文本、后台通知、自动更新与移动端不在该预览版内。不能宣称全部旧客户端已被等价重构。
