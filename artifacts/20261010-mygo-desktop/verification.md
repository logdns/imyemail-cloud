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

- Windows 首轮在 `gofmt` 门禁退出；已为 Go、Go manifests、shell、Python 固定 LF，并让 CI 输出失败文件名，不跳过格式检查。
- 最终 Go 源码在同一禁外网容器通过 vet 和 12 项 race 测试，新增“编辑账号 A 后切换账号 B 仍使用 A 发送”与“响应未知不自动重发、不丢弃正文”回归。Rust 源码未在本轮变更。
- RustSec `cargo-audit 0.22.2` 检查 295 个锁定依赖；2026-10-09 advisory 数据库未报告已知漏洞或警告。这不是 Go 依赖扫描或完整漏洞保证。
- CI 改为解压最终 ZIP/tar.gz 后执行版本和真实核心无窗口自检，并比较 ZIP/tar/DEB 内置核心与同次构建哈希。
- 次轮发现 Windows CP1252 默认编码不能读取含非 ASCII 的 Cargo UTF-8 元数据；已显式使用 UTF-8 并增加 Unicode 元数据/许可证回归，禁外网容器测试通过。
- Intel macOS 的未签核心经上游打包时 ad-hoc 签名，字节改变导致同次构建哈希检查正确拦截；修正为打包前明确签名并验证核心资源，使上游保留该签名，再比较最终包，不放宽哈希门禁。

- 安全审计限定为新增桌面版及其 IPC、凭据适配、打包/发布工作流。旧协议/移动端不属于完整覆盖；没有兼容的旧 coverage ledger。
- 完成 quick 单轮审查与独立最终覆盖复核：5 covered、8 deferred、2 out_of_scope，0 保留漏洞记录；两个结构 validator 通过。缺口和源码控制详见 [security-review.md](security-review.md)，不能描述为全面审计或无漏洞。
- 修正发送状态误报：不把返回 draft ID 当作已送达，不主动 flush 其他已排队消息；发送结果未知时提示先检查 outbox，不盲目重投。编辑中的发送账号固定，避免侧栏账号切换改变待发送动作。
- macOS 系统调用不能在本机满足完整硬资源限制，未在宿主账户执行审计目标；跨平台打包/包自检交给平台 CI，不声称真机或真实邮箱验收。
- 正式创建下载前必须等待新 MyGo 六架构 CI、原工程回归与限定范围审计；发布标签不可移动，下载附件必须独立复算 SHA-256。

## 最终源码门禁

- 源码：`7838430d77aa0d597d795bba2f99c968031aeb3b`；不可移动 annotated tag：`mygo-v0.3.0`。
- MyGo 六架构门禁 [38063232640](https://github.com/logdns/imyemail-cloud/actions/runs/38063232640)：6/6 成功，包含 fmt/vet、12 项 Go 测试、Unicode 许可证测试、Rust fmt/Clippy/5 项 desktop 测试、原生 Release 构建、解压包自检。Linux DEB 与 tar 内核心均检查数量和同次构建哈希，并分别自检。
- 原工程回归 [38063232604](https://github.com/logdns/imyemail-cloud/actions/runs/38063232604)：10/10 成功，Apple/iOS Simulator、Android、Windows、Linux、三系统 core、Push Bridge/testkit、仓库卫生均通过。
- 同一最终源码的 Linux arm64 在禁外网容器重新构建，DEB/tar 解压后的 `--version` 与 `--self-test` 均通过；两包内置核心哈希一致，HarfBuzz 单独 MIT notice 存在，DEB 显示独立 package 名/0.3.0/arm64 与 GTK/WebKitGTK/D-Bus 依赖。便携 tar 不包含独立安装脚本。
- 发布工作流 [38063935501](https://github.com/logdns/imyemail-cloud/actions/runs/38063935501) 从标签对应提交再次构建；最终发布和下载复验另行记录，不用分支上的包代替最终附件。

## 公开发布与下载复验

- [mygo-v0.3.0](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.0) 公开、非 draft、prerelease；发布工作流六个构建与 publish 作业全部成功。GitHub annotated tag 对象 `7f6eabeb1bab569d6e68eeeb534cef4344ffc164` 指向上述最终源码，未移动原 tag。
- 8 个最终包与各 `.sha256`、`SHA256SUMS` 共 17 个附件全部重新下载。禁外网、只读输入、512 MiB/1 CPU/32 PID/128 MiB scratch/90 秒墙钟容器独立检查：8/8 总表及独立哈希相符；Mach-O、PE、ELF 的 UI 与 helper 架构均正确；每包各一个主程序与核心。
- 所有主程序 Go build info 为 Go 1.27.1、固定 MyGo 版本、`vcs.revision=7838430d77aa0d597d795bba2f99c968031aeb3b`、`vcs.modified=false`，不是旧包。macOS Info.plist 为 `email.imy.cloud.mygo` / 0.3.0，并有 CodeResources；签名有效性由各 macOS 发布 CI 对解压包实际验证，不声称本机做了公证。
- Linux DEB 与 tar 内 helper SHA-256 按架构分别一致；DEB control 名称/版本/架构正确。每包均包含主许可证、HarfBuzz 单独 MIT notice 与 Rust 依赖声明。附件哈希见 [SHA256SUMS](SHA256SUMS)；源代码与检查通过不等于全部第三方许可证义务审计。
- 使用无 GitHub token 的 `curl -q` 匿名下载 macOS arm64 ZIP 和 SHA256SUMS，与上述下载逐字节一致，公开下载入口可用。
- GitHub Latest 仍为原 `v0.2.1`；Homebrew cask blob SHA 仍是 `15e601633588b71f1a25f4d71935e7c8ca16057d`，没有改动 tap。旧版 bundle 完整历史校验通过，`chckemail-old/` 仍被忽略。
- 发布后只提交下载状态、安装说明纠正和验证证据；不改产品源码、标签或已发布包，也不以证据提交触发新版本发布。

## 未验收边界

- Developer ID/Apple 公证、Authenticode、真实邮箱收发、GUI 真机/长期升级/回滚数据兼容性未验收。
- 新 UI 暂为英文；OAuth、附件、HTML 富文本、后台通知、自动更新与移动端不在该预览版内。不能宣称全部旧客户端已被等价重构。
