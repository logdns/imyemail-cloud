# 在 Linux 上安装 imyemail-cloud

imyemail-cloud 提供适用于 64 位 Intel/AMD（`amd64`）和 ARM（`arm64`）电脑的 Debian 安装包。

## 安装

双击下载的 `.deb`，使用系统“软件安装”打开；也可以在终端运行：

```bash
sudo apt install ./imyemail-cloud-linux-x86_64-20260919.deb
```

ARM 电脑请选择文件名包含 `arm64` 的安装包。`apt` 会安装 GTK、libadwaita、libsecret 和 WebKitGTK 等依赖，并把 imyemail-cloud 与正确图标注册到应用菜单。

安装完成后可从应用菜单启动，也可运行 `imyemail-cloud`。

## 验证下载

首次使用时导入发布公钥，然后验证独立签名：

```bash
gpg --import imyemail-cloud-release.asc
gpg --verify imyemail-cloud-linux-x86_64-20260919.deb.asc imyemail-cloud-linux-x86_64-20260919.deb
sha256sum -c imyemail-cloud-linux-x86_64-20260919.deb.sha256
```

主密钥完整指纹应为：

```text
713C 3AC1 D1EB D447 8CCF  4C70 56CD D5AB 9315 5F22
```

## 卸载

```bash
sudo apt remove imyemail-cloud
```

卸载软件不会自动删除本地邮件数据库、设置或系统凭据。远程图片仍默认阻止，可在“设置 → 安全与隐私”中开启。
