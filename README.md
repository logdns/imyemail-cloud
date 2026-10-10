# imyemail-cloud

[English](README.md) · [简体中文](README.zh-CN.md) · [Documentation](docs/README.md) · [Latest downloads](https://github.com/logdns/imyemail-cloud/releases/latest)

imyemail-cloud is an MIT-licensed, multi-platform native email client. The public source repository is [github.com/logdns/imyemail-cloud](https://github.com/logdns/imyemail-cloud), and all product links point to [https://imy.email](https://imy.email). The current version is `0.2.1`.

This repository contains only the native clients, shared mail core, optional push bridge, test tooling, build scripts, and open-source documentation. It contains no website/Web application, payments, subscriptions, software activation, device-seat limits, or commercial license gates. Client synchronization and sending do not require a purchased software license.

An additional [MyGo native desktop preview](desktop-mygo/README.md) is under development as version `0.3.0`, with a separate `mygo-v0.3.0` release track. It preserves the existing clients and Homebrew installation.

MyGo downloads use the separate [0.3.0 desktop preview release](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.0): macOS, Windows and Linux, each in amd64 and arm64 builds. See its [features, installation and signing limits](desktop-mygo/README.md); it is not a replacement for the full mobile/native feature set.

## Downloads

[Download the latest release](https://github.com/logdns/imyemail-cloud/releases/latest), or choose a `v0.2.1` development package directly. The links below are pinned to `v0.2.1`; use the Latest page for future versions.

### Homebrew (macOS Apple Silicon)

Trust the public tap once, then install with the short package name:

```bash
brew tap logdns/imyemail-cloud
brew trust --tap logdns/imyemail-cloud
brew install imyemail-cloud
```

Homebrew 7 requires one-time trust for third-party taps. After that use `brew upgrade imyemail-cloud` to update. See the [installation guide](docs/INSTALL.md) for all platforms, verification, uninstall and signing limits. The tap source is public at [logdns/homebrew-imyemail-cloud](https://github.com/logdns/homebrew-imyemail-cloud).

| Platform | Download |
| --- | --- |
| iOS/iPadOS arm64 | [unsigned IPA](https://github.com/logdns/imyemail-cloud/releases/download/v0.2.1/imyemail-cloud-ios-arm64-20261002-unsigned.ipa) (requires your own valid Apple certificate and provisioning profile) |
| Android | [unsigned APK](https://github.com/logdns/imyemail-cloud/releases/download/v0.2.1/imyemail-cloud-android-20261002-unsigned.apk) (for development or self-signing; not a signed store package) |
| macOS Apple Silicon | [Homebrew tap](https://github.com/logdns/homebrew-imyemail-cloud) or [ad-hoc signed ZIP](https://github.com/logdns/imyemail-cloud/releases/download/v0.2.1/imyemail-cloud-macos-arm64-20261002-unsigned.zip) |
| Linux x86_64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/v0.2.1/imyemail-cloud-linux-x86_64-20261002.deb) |
| Linux ARM64 | [DEB](https://github.com/logdns/imyemail-cloud/releases/download/v0.2.1/imyemail-cloud-linux-arm64-20261002.deb) |
| Windows x64 | [unsigned installer](https://github.com/logdns/imyemail-cloud/releases/download/v0.2.1/imyemail-cloud-windows-x64-20261002-setup.exe) |
| Windows ARM64 | [unsigned installer](https://github.com/logdns/imyemail-cloud/releases/download/v0.2.1/imyemail-cloud-windows-arm64-20261002-setup.exe) |

Verify these downloads with the matching [v0.2.1 SHA256SUMS](https://github.com/logdns/imyemail-cloud/releases/download/v0.2.1/SHA256SUMS). These are unsigned development packages. See the [release documentation](docs/RELEASE.md) for signing, notarization, real-device, and store-validation boundaries.

## Repository layout

| Directory | Contents |
| --- | --- |
| `core/` | Rust IMAP, SMTP, MIME, synchronization, SQLite, HTML sanitization, and native interfaces |
| `desktop-mygo/` | Additional native Go desktop preview, bundled Rust stdio core and six-architecture packaging |
| `apple/` | Native macOS, iOS, and iPadOS SwiftUI clients |
| `android/` | Native Android Jetpack Compose client |
| `linux/` | Native Linux GTK4/libadwaita client |
| `windows/` | Native Windows WinUI 3 client |
| `push-bridge/` | Optional signed webhook push bridge |
| `email-testkit/` | Isolated IMAP/SMTP test fixtures |
| `brand/` | Client brand sources and generation scripts |
| `docs/` | Architecture, development, security, and release documentation |

## Quick start

Shared core:

```bash
cd core
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --locked --release -p chck-cli -p chck-ffi-c
```

Platform prerequisites and commands are documented in the [Apple](apple/README.md), [Android](android/README.md), [Linux](linux/README.md), and [Windows](windows/README.md) guides. See [development](docs/DEVELOPMENT.md), [architecture](docs/ARCHITECTURE.md), and [release](docs/RELEASE.md) documentation for the full workflow.

## Release boundary

CI tests the shared core and each platform project, then creates development artifacts. GitHub Releases provides public downloads clearly marked as unsigned development packages. They are not Apple-notarized builds, signed Android AABs, Authenticode-signed Windows installers, or app-store releases.

## Contributing and security

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing. Report security issues privately as described in [SECURITY.md](SECURITY.md); do not disclose unpatched vulnerabilities in a public issue.

## License

[MIT](LICENSE)
