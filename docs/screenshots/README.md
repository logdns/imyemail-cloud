# UI screenshots / 软件界面截图

Captured 2026-10-11. Native source: `9738a2e393ca31c469cea87ca4ffa45682745023`, UI unchanged in the new release. MyGo 0.3.2 source: the Pulse-inspired `desktop-mygo/design.go`, `view.go`, `view_test.go` and version metadata committed alongside these images; isolated Linux renderer capture after Go vet/race tests. Unedited PNGs, actual UI with synthetic `.test` mail only; no private data, desktop capture or AI mockup.

| Image | Capture | Dimensions | SHA-256 |
| --- | --- | --- | --- |
| [imyemail-cloud-native.png](imyemail-cloud-native.png) | macOS SwiftUI Preview; [CI 38068249840](https://github.com/logdns/imyemail-cloud/actions/runs/38068249840), artifact `screenshot-native` | 1180 × 684 | `a58509cd3163954f8b75a0b03057ed2a2c87c3812226a64ed86afe7ba09dffea` |
| [imyemail-cloud-mygo.png](imyemail-cloud-mygo.png) | MyGo headless renderer, `TestReadmeScreenshot`, Pulse-inspired light layout | 1120 × 760 | `3b3df238be17a3e0c338df4783889b7763039b3e3c2fb1ad90a10b1984a757f4` |
| [imyemail-cloud-mygo-dark.png](imyemail-cloud-mygo-dark.png) | Same renderer/source/fixture, dark appearance | 1120 × 760 | `baec3c8d5d2b80dadb69fc038bb505e539b4bffab6f2ed3e0e488b84e79248e7` |

![MyGo dark appearance](imyemail-cloud-mygo-dark.png)

Native uses `IMYEMAIL_CLOUD_PREVIEW=1`, `IMYEMAIL_CLOUD_CAPTURE_SCENE=modern` and the app's own view capture; MyGo uses `IMYEMAIL_CLOUD_CAPTURE_PATH=<scratch>/imyemail-cloud-mygo.png go test -run '^TestReadmeScreenshot$'`. Capture hooks are opt-in, not live-mail startup paths. These images show UI layout, not feature parity, actual email delivery, rich HTML capture, OS trust, GPU/window-manager or real-device acceptance.

原生版展示 macOS 预览 UI；MyGo 展示真实 UI 渲染器的合成邮件夹具。没有编辑拼接、真实邮箱或 AI 生成图片；不把截图当成收发、功能等价或真机验收证据。
