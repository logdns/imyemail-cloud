# push-bridge

双版本客户端入口：[安装](../docs/INSTALL.zh-CN.md)、[验签](../docs/SIGNING.md)、[历史版本](../docs/VERSIONS.md)。该桥为可选服务，不随任一桌面安装自动部署。

imyemail 签名 Webhook → APNs/FCM。推送只含 `account_id` / `folder` / `count`。

```bash
IMYEMAIL_CLOUD_WEBHOOK_SECRET=dev-secret cargo run
# POST /  Header X-Imyemail-Cloud-Signature: <hex hmac-sha256>
```

与 imyemail 同机 Docker 部署，仅内网入站 + 出站 443。
