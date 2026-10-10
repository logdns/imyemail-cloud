# Install: choose an edition

[English](INSTALL.md) · [简体中文](INSTALL.zh-CN.md) · [Signatures](SIGNING.md) · [Preserved versions](VERSIONS.md)

Both editions remain independently maintained in the [same public repository](https://github.com/logdns/imyemail-cloud). No automatic account migration, replacement of the other edition, or data deletion.

| Edition | Download | Platforms | Previous releases retained |
| --- | --- | --- | --- |
| `imyemail-cloud-native` 0.2.4 | [native-v0.2.4](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.4) | macOS arm64, iOS/iPadOS, Android, Linux/Windows x64/ARM64 | [v0.2.1](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.1), [v0.2.0](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.0) |
| `imyemail-cloud-mygo` 0.3.4 preview | [mygo-v0.3.4](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.4) | macOS/Windows/Linux amd64/arm64 | [mygo-v0.3.0](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.0) |

MyGo is a Go-native desktop preview, not a web client or feature-equivalent mobile replacement. See [features and vault requirements](../desktop-mygo/README.md). GitHub Latest belongs to the original track; **select MyGo by its explicit tag**.

## Verify before installing

From the **same tagged release**, download your package, its `.sha256`, `SHA256SUMS`, `SHA256SUMS.sig`, `release-cert.pem` and `release-public.pem`. Follow [SIGNING.md](SIGNING.md): pin the complete certificate fingerprint, verify the RSA/SHA-256 manifest signature, then compare the package hash. A public key downloaded next to a package is not independent authentication unless compared with a previously trusted fingerprint.

macOS/Windows packages use a stable **self-signed code-signing certificate**, Android uses a stable app-signing key. Linux uses the signed checksum manifest, **not an APT repository signature**. Self-signed does not mean publicly trusted, Apple-notarized, store-approved or real-device validated. iOS remains unsigned.

## macOS 14+

Homebrew 7 requires one-time trust of the third-party tap:

```bash
brew tap logdns/imyemail-cloud
brew trust --tap logdns/imyemail-cloud
brew install imyemail-cloud-native  # native, Apple Silicon
brew install imyemail-cloud-mygo  # optional separate preview, Intel / Apple Silicon
```

Update the chosen token with `brew update && brew upgrade <token>`; uninstall with `HOMEBREW_NO_INSTALL_CLEANUP=1 brew uninstall <token>`. Normal uninstall retains databases and Keychain credentials. The [public tap](https://github.com/logdns/homebrew-imyemail-cloud) pins release URLs and SHA-256 independently; the original token never switches to MyGo.

Manual installation: choose `*-selfsigned.zip`, extract and copy only the matching app (`imyemail-cloud-native.app` or `imyemail-cloud-mygo.app`) to Applications. Verify its signature using SIGNING.md. Gatekeeper may reject self-signed apps; use normal System Settings > Privacy & Security approval only if you trust the verified publisher. No guarantee of Gatekeeper acceptance; do not disable it or remove quarantine globally.

## Windows

Original: select x64/ARM64 `*-selfsigned-setup.exe` and verify before running; requires Windows 10 1809+/11 and WebView2. MyGo: extract the matching `*-windows-amd64-selfsigned.zip` or `*-windows-arm64-selfsigned.zip` into a separate user-owned directory; run `imyemail-cloud-mygo.exe`, keeping its core and licenses beside it. This ZIP is portable, not an installer/MSIX.

The self-signed publisher is untrusted by default; SmartScreen reputation is separate. SIGNING.md provides optional fingerprint-pinned **CurrentUser-only** certificate trust and removal. Never trust arbitrary roots or disable SmartScreen.

## Android (original only)

Select `imyemail-cloud-native-android-20261011-selfsigned.apk`, verify the hash and APK signer, then allow installation from your chosen download app if desired. Android 8+, arm64-v8a/armeabi-v7a/x86_64 cores. No Play Store/device acceptance is implied.

Updates require the **same signing key** and increased versionCode (0.2.4 uses 6). Debug, privately re-signed or historical Chck APKs may have another signer and cannot upgrade in place. Back up data before any uninstall; never delete accounts merely to bypass a signer mismatch.

## Linux

Native: x86_64/ARM64 DEB. MyGo: amd64/arm64 DEB or tar.gz, built on Ubuntu 24.04. After verification, `sudo apt install ./<package>.deb`. Remove only the chosen package with `sudo apt remove imyemail-cloud-native` or `sudo apt remove imyemail-cloud-mygo`, not purge/autoremove. Extract the entire MyGo tar directory and run its binary in place, retaining helper/licenses.

MyGo requires compatible runtime libraries, GTK 3, D-Bus and an unlocked Secret Service; no plaintext-secret fallback. Upstream DEB additionally declares WebKitGTK although this UI does not render mail HTML. Original GTK4 has its own dependencies. Do not reuse another version's install scripts.

## iOS/iPadOS (original only)

`*-unsigned.ipa` is an arm64 build/signing input, not directly installable. Requires a valid Apple certificate, matching App ID, provisioning profile and permitted device. A locally generated self-signed certificate cannot replace Apple provisioning. No TestFlight/App Store release.

## Update, coexistence and rollback

Close the selected app before replacing binaries and back up its database **and OS-vault credentials**. Original ID: `email.imy.cloud`; MyGo: `email.imy.cloud.mygo`, separate `imyemail-cloud-mygo` config directory. No automatic migration or binary updater.

Old tags/packages remain available. Rollback uses the historical package **and matching manifest**; restore a compatible pre-upgrade data backup if schema compatibility is unknown. Android normally rejects lower versionCode; prefer a forward fix. [VERSIONS.md](VERSIONS.md) covers source restoration and future independent releases.

### Existing native installations

The native rename preserves app IDs, profiles, OS-vault services and the Windows installer upgrade identity. Close the old app first. Do not run the old and renamed native app together: they share the same database. Never remove mail data as part of the rename.

For legacy Homebrew users, first back up the native profile and Keychain, then run `HOMEBREW_NO_INSTALL_CLEANUP=1 brew uninstall --cask imyemail-cloud` (no `--zap`) followed by `brew install imyemail-cloud-native`. The old cask stays pinned to its historical package and conflicts with the new native cask, not MyGo. Linux native DEB uses version-bounded Breaks/Replaces for the old `imyemail-cloud` package; review APT's transaction before confirming. The internal native CLI/Windows executable name remains `imyemail-cloud` for compatibility; the new Homebrew CLI and Linux alias are `imyemail-cloud-native`.
