# Linux 安装：原版与 MyGo

[统一安装](https://github.com/logdns/imyemail-cloud/blob/main/docs/INSTALL.zh-CN.md) · [自签校验](https://github.com/logdns/imyemail-cloud/blob/main/docs/SIGNING.md) · [历史版本/更新](https://github.com/logdns/imyemail-cloud/blob/main/docs/VERSIONS.md)

原生版 0.2.2：[native-v0.2.2](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.2)，x86_64/ARM64 DEB；MyGo 0.3.1：[mygo-v0.3.1](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.1)，amd64/arm64 DEB 或 tar.gz。旧包保留，包与校验清单不能跨标签混用。

先固定 RSA 证书指纹，验证 SHA256SUMS 签名和包哈希，再 `sudo apt install ./对应包.deb`。这是签名清单认证，不是 APT 仓库签名；历史 GPG 密钥/命令不适用于这批包。

原版需 GTK4/libadwaita/libsecret/WebKitGTK；MyGo 需兼容 Ubuntu 24.04 运行库、GTK 3、D-Bus、解锁的 Secret Service，保留核心和许可证。应用/数据独立，按架构选择。

运行 `imyemail-cloud-native` 或 `imyemail-cloud-mygo`；卸载仅用 `sudo apt remove 对应包名`，不要 purge/autoremove、删除邮件或钥匙库。原生兼容 CLI `imyemail-cloud` 和账号数据位置保持不变。更新前备份数据库及系统凭据，回滚遵循对应版本说明。
