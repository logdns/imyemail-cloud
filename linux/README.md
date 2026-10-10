# imyemail-cloud-native for Linux

GTK4 + libadwaita 原生客户端，App ID 为 `email.imy.cloud`。邮件逻辑直接链接同仓库 `core/`。

原版 `0.2.2` 与 MyGo `0.3.1` 独立包名/数据目录，历史版本保留。[安装/卸载](../docs/INSTALL.zh-CN.md) · [签名清单校验](../docs/SIGNING.md) · [后续更新与回滚](../docs/VERSIONS.md)。Linux 采用签名清单认证，不声称 APT 仓库签名。

## 基础验证

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo run -- providers
```

## GTK 构建

Debian/Ubuntu 依赖：

```bash
sudo apt install libgtk-4-dev libadwaita-1-dev libwebkitgtk-6.0-dev libsecret-1-dev pkg-config
cargo clippy --locked --all-targets --features gtk,secret,webkit -- -D warnings
cargo test --locked --features gtk,secret,webkit
./scripts/package-deb.sh
```

`secret` feature 使用 libsecret；没有该 feature 时会使用权限为 `0600` 的本地 sidecar。`webkit` feature 显示已净化 HTML，禁用脚本并默认阻止远程图片。

DEB 构建、安装和桌面启动的完整验收由 Linux CI 执行。macOS 上的无 GTK 测试不能替代该验收。
