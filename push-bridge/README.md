# push-bridge

imyemail 签名 Webhook → APNs/FCM。推送只含 `account_id` / `folder` / `count`。

```bash
IMYEMAIL_CLOUD_WEBHOOK_SECRET=dev-secret cargo run
# POST /  Header X-Imyemail-Cloud-Signature: <hex hmac-sha256>
```

与 imyemail 同机 Docker 部署，仅内网入站 + 出站 443。
