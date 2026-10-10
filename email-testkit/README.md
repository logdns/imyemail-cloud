# email-testkit

客户端[安装/下载](../docs/INSTALL.zh-CN.md)、[验签](../docs/SIGNING.md)、[双版本保留](../docs/VERSIONS.md)。测试邮局不是生产邮件服务，不随客户端自动安装。

协议 golden 语料、畸形 MIME 样本、本地 Docker 测试邮局。`core` 与 Windows MailKit 适配层共用同一套语料。

## 布局

```text
email-testkit/
├── docker-compose.yml      # 一键测试邮局（GreenMail）
├── golden/                 # 协议/MIME 期望输出
│   └── mime/
├── fixtures/               # 原始 .eml 样本
└── seeds/                  # 导入邮局的种子邮件
```

## 启动测试邮局

```bash
docker compose up -d
```

默认账号（IMAP 3143 / SMTP 3025，容器内为明文，仅限本机测试）：

| 邮箱 | 密码 |
|---|---|
| `dev@imyemail.test` | `devpass` |
| `alice@imyemail.test` | `alicepass` |

CI 禁止依赖公网邮箱。OAuth 回归不进默认流水线。

GreenMail 是 M0 占位；M1 集成测试稳定后可替换为 imyemail Compose（见产品文档 04 / 12 章）。
