# 文档索引

- [INSTALL.md](INSTALL.md) / [INSTALL.zh-CN.md](INSTALL.zh-CN.md)：各平台下载、Homebrew 安装与校验
- [SIGNING.md](SIGNING.md)：固定自签指纹、清单/代码/APK 验签、信任移除与密钥保留
- [VERSIONS.md](VERSIONS.md)：两条版本线、旧源码恢复、后续独立维护与回滚
- [界面截图](screenshots/README.md)：两版真实 UI 的合成邮件截图、来源和哈希
- [双版本发布验证](../artifacts/20261011-self-signed-tracks/verification.md)：命名兼容、签名、测试、下载和 Homebrew 同步
- [ARCHITECTURE.md](ARCHITECTURE.md)：仓库结构、运行边界和数据流
- [DEVELOPMENT.md](DEVELOPMENT.md)：环境、测试和开发规则
- [RELEASE.md](RELEASE.md)：版本、构建、GitHub Release 和签名边界
- [SECURITY.md](SECURITY.md)：客户端安全模型和检查清单
- [MYGO.zh-CN.md](MYGO.zh-CN.md)：Go 原生桌面重构版、六架构下载、功能边界与独立预览发布
- [MyGo 验证记录](../artifacts/20261010-mygo-desktop/verification.md)：隔离测试、限定范围审计、构建与下载核验
- [MyGo 安全审查摘要](../artifacts/20261010-mygo-desktop/security-review.md)：源码控制、partial coverage 与待验收边界
- [开源重构验证记录](../artifacts/20261002-open-source-refactor/verification.md)：本地门禁、聚焦安全检查和未验收边界
- [Homebrew tap 发布验证](../artifacts/20261002-homebrew-tap/verification.md)：cask、安装、供应链校验和发布边界

当前原生版 `imyemail-cloud-native`：`0.2.4` / `native-v0.2.4`；MyGo `imyemail-cloud-mygo`：`0.3.4` / `mygo-v0.3.4`。两版分别自签发布，旧 `v0.2.0/v0.2.1/mygo-v0.3.0` 保留。项目主页：[https://imy.email](https://imy.email)，源码：[github.com/logdns/imyemail-cloud](https://github.com/logdns/imyemail-cloud)。

已发布并复验全部 15 个下载包。原生回归 10/10、MyGo 六架构 6/6、[双 cask 安装/验签/共存/卸载 3/3](https://github.com/logdns/homebrew-imyemail-cloud/actions/runs/38078815415) 与[双版本签名同步](https://github.com/logdns/homebrew-imyemail-cloud/actions/runs/38078815748)通过；实际源码、签名和审查缺口以[本轮验证记录](../artifacts/20261011-self-signed-tracks/verification.md)为准，不代表真机、真实邮箱、公证或商店验收。

公开历史版本在 Git 标签/Release 中保留；原始 Chck 资料另存本地 `chckemail-old/`，被 Git 忽略，不属于公开仓库。日期证据按历史事实保留。
