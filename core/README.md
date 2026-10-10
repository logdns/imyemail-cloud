# imyemail-cloud core

共享核心 `0.2.2` 同时服务原版和独立 MyGo 桌面版。[下载/安装](../docs/INSTALL.zh-CN.md) · [签名](../docs/SIGNING.md) · [双版本与历史恢复](../docs/VERSIONS.md)。

跨平台共享 Rust 邮件核心，负责 IMAP/SMTP、MIME、同步、SQLite、本地搜索、HTML 净化、服务商预设及 C/JSON 接口。客户端 UI 不重复实现协议逻辑。

现有 `chck-*` crate 和 ABI 名称是源码兼容标识，不代表对外产品品牌；对外产品名、链接和包信息均为 `imyemail-cloud`。

## Crates

| Crate | 职责 |
| --- | --- |
| `chck-types` | 领域模型和错误类型 |
| `chck-providers` | 邮件服务商预设 |
| `chck-sanitize` | HTML 净化和远程资源策略 |
| `chck-mime` | MIME 解析与编码 |
| `chck-proto-imap` | IMAP、IDLE、TLS |
| `chck-proto-smtp` | SMTP Submission、TLS |
| `chck-store` | SQLite schema、迁移、FTS |
| `chck-sync` | 同步和本地操作队列 |
| `chck-imyemail` | imyemail 能力探测 |
| `chck-engine` | 客户端统一门面 |
| `chck-ffi` / `chck-ffi-c` | 原生客户端接口 |
| `chck-cli` | 调试与自动化 CLI |

## 验证

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --locked --release -p chck-cli -p chck-ffi-c
```

核心没有软件许可或收费门禁。邮箱服务商要求的密码、应用密码或授权码仍属于邮箱登录凭据。

License: [MIT](LICENSE)
