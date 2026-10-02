MailKit 适配层实现 `IMailEngine`，与 `core` 共用 `providers.json` 与 schema v3。

- IMAP 993 / SMTP 465 强制 TLS；587 STARTTLS（Outlook/iCloud）
- QQ/网易 quirks：LOGIN 前发 IMAP `ID`
- Windows 新凭据写入 Credential Locker；旧 `{mail.db}.secrets` 仅迁移，不做新凭据明文降级。非 Windows 测试仍可使用 sidecar；密码不进 SQLite。
