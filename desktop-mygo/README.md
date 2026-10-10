# imyemail-cloud — MyGo desktop edition

[Project](../README.md) · [中文说明](../docs/MYGO.zh-CN.md) · [Source framework](https://github.com/egoist/mygo)

This is a separate **0.3.0 desktop preview**, using MyGo's Go-native GPU UI and the existing Rust mail core. It does not replace the original Apple, Android, Linux or Windows clients, their databases, or the `imyemail-cloud` Homebrew cask. MyGo is MIT licensed and pinned to commit `b8beccc577daa00fed7c24112b0a7a0550d4825b` in `go.mod`/`go.sum`.

## Features and boundaries

- Manual IMAP account setup with app passwords, verified TLS, account/folder selection and synchronization.
- Cached inbox, subject/sender filtering and **plain-text** message reading; no HTML, scripts, images or remote resources are rendered.
- Plain-text drafts and sending; the UI checks outbox state and never equates a successful RPC with recipient delivery.
- Passwords travel over private process pipes, never command-line arguments. Storage uses macOS Keychain, Windows native credentials or Linux Secret Service. No plaintext credential fallback; a locked or unavailable vault makes account operations fail visibly.
- The private database uses a separate `imyemail-cloud-mygo` directory under the OS user-config directory. No old-client database or credential migration is performed. Normal installation does not remove old mail data.

This first slice has no OAuth browser flow, attachment import/export, rich HTML reading/composition, background notifications, mobile UI or automatic binary updater. It is not feature-equivalent to all existing clients. The UI currently uses English labels; Chinese operating instructions are linked above.

## Build and test

Requirements: Go **1.27.1**, Rust **1.95.0** and the pinned module dependencies. Linux packaging requires GTK 3, D-Bus development headers and pkg-config; runtime account operations require an unlocked Secret Service. macOS packaging requires Xcode command-line tools. Windows portable packages are unsigned.

```bash
cd desktop-mygo
go mod download
go vet ./...
go test -count=1 ./...
bash scripts/package.sh darwin/arm64
```

`package.sh` builds the current platform only (`darwin/amd64`, `linux/amd64`, `linux/arm64`, `windows/amd64`, `windows/arm64` also supported), invokes the pinned `go tool mygo build`, bundles the same-build Rust engine and license notices, and generates packages with SHA-256 under `dist/release/`. Version metadata comes from `mygo.json`.

Tests use fixture engines and MyGo's headless UI tester. Audit execution requires isolated networking, environment, files and resource limits; do not use real mailboxes or the maintainer's Keychain for fixture tests.

Packaged executables support `--version` and `--self-test --data-dir <absolute-empty-directory>`. The self-test only lists accounts and builds a headless UI; it does not authenticate or send mail. Development overrides `--engine <absolute-path>` and `--data-dir <absolute-path>` are explicit CLI options, not untrusted message inputs.

## Downloads and installation

Tagged download entry: [MyGo 0.3.0 desktop preview](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.0). Download availability is conditional on the release gates below; the original client remains [v0.2.1](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.1).

The new workflow `.github/workflows/mygo-desktop.yml` tests and packages six desktop OS/architecture combinations. Only a manually requested, verified, tagged build publishes a separate `mygo-v0.3.0` preview release in the existing `logdns/imyemail-cloud` repository. The original `v0.2.1` remains available; do not replace its assets or silently switch the Homebrew tap.

- macOS: extract the ad-hoc-signed ZIP and copy `imyemail-cloud-mygo.app` to Applications. It is **not Apple-notarized**; normal Gatekeeper approval remains required.
- Windows: extract the unsigned portable ZIP into a user-owned directory, retaining the bundled `imyemail-cloud-core.exe`, then run `imyemail-cloud-mygo.exe`. Authenticode and installation/upgrade behavior need separate validation.
- Linux: use the matching DEB or extract the archive and review its install script. GTK 3 and a functioning Secret Service are required; native GPU/runtime validation is separate from the headless tests.

Download the matching `.sha256` or `SHA256SUMS` from the **same tagged release** and recompute before installation. No unsigned auto-updater is enabled.

On macOS/Linux, run `shasum -a 256 <downloaded-package>` or `sha256sum -c <downloaded-package>.sha256`. On Windows, run `Get-FileHash <downloaded-package> -Algorithm SHA256` in PowerShell. Compare the full digest, not a filename or size. Hashes establish download consistency; ad-hoc/unsigned packages do not authenticate the publisher independently.
