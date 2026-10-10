# imyemail-cloud-native for Android

Kotlin 2.1、Jetpack Compose Material 3 原生客户端。`applicationId` 为 `email.imy.cloud`，最低 Android 8（API 26），目标 API 35。协议和存储通过 JNI 连接同仓库 `core/`。

原版 `0.2.2` 发布固定密钥签名 APK（versionCode 4）；MyGo 仅桌面，不替代 Android。[安装与更新](../docs/INSTALL.zh-CN.md) · [APK 指纹与密钥连续性](../docs/SIGNING.md) · [旧版本保留](../docs/VERSIONS.md)。不得用不同签名 APK 静默覆盖旧账号。

## 验证

需要 JDK 17、Android SDK 35、NDK 27.2 和 Python 3.11。

```bash
./scripts/check-environment.sh
./scripts/build-native.sh
./gradlew :core:common:test :core:data:test :feature:thread:testDebugUnitTest \
  :app:lintDebug :app:assembleDebug --no-daemon
```

完整设备验证入口：

```bash
./scripts/verify-android.sh
```

APK 必须包含目标 ABI 的 `libchck_mail.so` 并在设备内成功加载。JVM 测试或 Compose Preview 不能代替 JNI、Keystore、设备安装和真实收发验证。

凭据使用 Android Keystore 保护。正文 WebView 禁用脚本，限制导航，并默认阻止远程内容。邮箱服务商的应用密码/授权码是登录凭据，不是软件许可证。
