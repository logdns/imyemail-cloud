# Install imyemail-cloud

[English](INSTALL.md) · [简体中文](INSTALL.zh-CN.md) · [Latest Release](https://github.com/logdns/imyemail-cloud/releases/latest)

## macOS (Apple Silicon, macOS 14+)

The public Homebrew tap installs the application in `/Applications` and links its command-line executable. On each new machine:

```bash
brew tap logdns/imyemail-cloud
brew trust --tap logdns/imyemail-cloud
brew install imyemail-cloud
```

Homebrew 7 requires one-time trust for third-party taps. Subsequent installs need only `brew install imyemail-cloud`. To update, run `brew update && brew upgrade imyemail-cloud`; to remove the Homebrew-managed application and CLI link, run `brew uninstall imyemail-cloud`. Normal uninstall does not remove mailbox data or Keychain credentials.

The [tap source](https://github.com/logdns/homebrew-imyemail-cloud) pins the release ZIP and SHA-256. Alternatively, download the macOS ZIP from the [Latest Release](https://github.com/logdns/imyemail-cloud/releases/latest). The current build is ad-hoc signed but **not Apple-notarized**; Gatekeeper may require approval in System Settings > Privacy & Security. SHA-256 verification is not notarization.

## Other platforms

Choose the matching asset from the [Latest Release](https://github.com/logdns/imyemail-cloud/releases/latest):

| Platform | Asset type | Limitation |
| --- | --- | --- |
| iOS/iPadOS arm64 | IPA | Unsigned; requires your own Apple certificate and provisioning profile before installation. Not a TestFlight/App Store package. |
| Android | APK | Unsigned Release input; requires signing before device or store distribution. |
| Linux x86_64 / ARM64 | DEB | No distribution-repository signing. |
| Windows x64 / ARM64 | EXE installer | No Authenticode signature. |

Download `SHA256SUMS` **from the same tagged release** as the packages and check the files you downloaded (macOS: `shasum -a 256 -c SHA256SUMS`; Linux: `sha256sum -c SHA256SUMS`). The aggregate file lists every asset, so checking only a subset will report missing files; in that case use the matching per-asset `.sha256` from the same release instead. Do not mix Latest packages with a different version's checksum list. For signing and device-validation limits, see [release policy](RELEASE.md).
