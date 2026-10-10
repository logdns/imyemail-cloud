# 安全设计

[双版本安装与校验](INSTALL.zh-CN.md) · [自签证书/密钥边界](SIGNING.md) · [版本保留](VERSIONS.md)。自签不等于公信、公证、商店或真机验收；历史审计只覆盖其记录的源码与范围。

MyGo 使用独立账号/系统凭据服务与私有 stdio helper，不监听本地 HTTP；邮件仅进入只读纯文本 UI，不启用 HTML/WebView/远程图片。Pulse 借鉴只涉及自有 UI 组件的视觉布局，不增加邮件执行或跨版本迁移能力。原生客户端仍需逐平台审查净化与 WebView 边界。

当前发布、身份兼容、Pulse UI 与 Homebrew 同步为[限定范围增量复核](../artifacts/20261011-self-signed-tracks/verification.md)；先前 MyGo [quick 审查](../artifacts/20261010-mygo-desktop/security-review.md)有明确延后项。两者均不应称为完整协议/上游框架审计或“无漏洞”。真实邮箱、OS 凭据 ACL、GPU/真机及长期升级恢复仍待专门验收。

## 主要信任边界

- 不可信邮件 MIME/HTML/URL → 解析器、净化器和平台 WebView；
- 邮件服务商网络 → IMAP/SMTP/OAuth/TLS 和同步队列；
- 本地输入、附件和 deep link → 原生 UI、FFI/JNI 和文件系统；
- 邮箱凭据 → 操作系统凭据库及受限的迁移 sidecar；
- 发布输入 → 编译产物、清单、哈希、签名和 GitHub Release。

## 不变量

- 禁止把邮箱密码、token 或真实邮件写入日志、仓库或发布包。
- WebView 禁用脚本、权限、下载和非预期导航；远程内容默认阻止。
- TLS 校验默认开启，测试证书例外只能用于明确的本地夹具。
- FFI/JNI 句柄必须遵守生命周期、并发和释放约定。
- 附件和导入文件必须有大小、类型、超时和临时文件清理限制。
- 软件没有收费授权或激活接口，不能重新引入功能门禁。
- 发布包必须记录源码提交、版本、架构、哈希和签名边界。

漏洞报告方式见仓库根目录 [SECURITY.md](../SECURITY.md)。
