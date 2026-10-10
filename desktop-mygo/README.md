# imyemail-cloud-mygo — Go-native desktop edition

[Project](../README.md) · [中文说明](../docs/MYGO.zh-CN.md) · [Source framework](https://github.com/egoist/mygo)

This is a separate **0.3.1 self-signed desktop preview**, using MyGo's Go-native GPU UI and the existing Rust mail core. It does not replace `imyemail-cloud-native`, its data or its independent Homebrew cask. Its own token is `imyemail-cloud-mygo`. MyGo is MIT licensed and pinned to commit `b8beccc577daa00fed7c24112b0a7a0550d4825b` in `go.mod`/`go.sum`. [Install both editions](../docs/INSTALL.md) · [Signatures](../docs/SIGNING.md) · [Preserved versions](../docs/VERSIONS.md).

## Features and boundaries

### Pulse-inspired native layout

The visual hierarchy borrows from [Pulse](https://pulse.egoist.dev/): a quiet gray sidebar, blue selection, original colored outline icons, rounded summary cards and a focused mail-reading panel. It is implemented with our own MyGo components, not Pulse branding/assets or a Web UI. Counts describe the loaded cache/folders, not server totals or real-time statistics. Light/dark follows system appearance; narrow windows omit summary cards to preserve controls. [UI captures](../docs/screenshots/README.md) use synthetic mail, not a live account.

- Manual IMAP account setup with app passwords, verified TLS, account/folder selection and synchronization.
- Cached inbox, subject/sender filtering and **plain-text** message reading; no HTML, scripts, images or remote resources are rendered.
- Plain-text drafts and sending; the UI checks outbox state and never equates a successful RPC with recipient delivery.
- Passwords travel over private process pipes, never command-line arguments. Storage uses macOS Keychain, Windows native credentials or Linux Secret Service. No plaintext credential fallback; a locked or unavailable vault makes account operations fail visibly.
- The private database uses a separate `imyemail-cloud-mygo` directory under the OS user-config directory. No old-client database or credential migration is performed. Normal installation does not remove old mail data.

This first slice has no OAuth browser flow, attachment import/export, rich HTML reading/composition, background notifications, mobile UI or automatic binary updater. It is not feature-equivalent to all existing clients. The UI currently uses English labels; Chinese operating instructions are linked above.

## Build and test

Requirements: Go **1.27.1**, Rust **1.95.0** and pinned dependencies. Linux requires GTK 3/D-Bus/pkg-config and an unlocked Secret Service. macOS requires Xcode command-line tools. Local builds default to ad-hoc macOS/unsigned Windows; tagged manual CI publication uses the stable self-signed identity.

```bash
cd desktop-mygo
go mod download
go vet ./...
go test -count=1 ./...
RUSTUP_TOOLCHAIN=1.95.0 bash scripts/package.sh darwin/arm64
```

`package.sh` builds the current platform only (`darwin/amd64`, `linux/amd64`, `linux/arm64`, `windows/amd64`, `windows/arm64` also supported), invokes the pinned `go tool mygo build`, bundles the same-build Rust engine and license notices, and generates packages with SHA-256 under `dist/release/`. Version metadata comes from `mygo.json`.

Tests use fixture engines and MyGo's headless UI tester. Audit execution requires isolated networking, environment, files and resource limits; do not use real mailboxes or the maintainer's Keychain for fixture tests.

Packaged executables support `--version` and `--self-test --data-dir <absolute-empty-directory>`. The self-test only lists accounts and builds a headless UI; it does not authenticate or send mail. Development overrides `--engine <absolute-path>` and `--data-dir <absolute-path>` are explicit CLI options, not untrusted message inputs.

## Downloads and installation

Download: [MyGo 0.3.1 self-signed preview](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.1), six architectures/eight packages with signed SHA256SUMS. Native [0.2.2](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.2) is independent. Historical [mygo-v0.3.0](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.0) and [v0.2.1](https://github.com/logdns/imyemail-cloud/releases/tag/v0.2.1) remain intact.

`.github/workflows/mygo-desktop.yml` tests/packages six combinations; only a manually requested verified tagged build signs and publishes. No release overwrite or cask switch. Installation, update/uninstall and rollback instructions are centralized in [INSTALL.md](../docs/INSTALL.md).

- macOS: extract the `*-selfsigned.zip` and copy `imyemail-cloud-mygo.app` to Applications, or `brew install imyemail-cloud-mygo` after tap/trust. **Not Apple-notarized**; Gatekeeper trust is separate.
- Windows: extract the `*-selfsigned.zip` into a user-owned directory, retaining `imyemail-cloud-core.exe`, then run `imyemail-cloud-mygo.exe`. Both have self-signed Authenticode; no public trust/SmartScreen reputation is claimed.
- Linux: packages are built on Ubuntu 24.04; use that release or a compatible runtime. Use the matching DEB, or extract the entire portable tar.gz and run `imyemail-cloud-mygo` in that directory, keeping its core and licenses beside it. The portable release archive does not include the toolkit's separate install/uninstall script. GTK 3, D-Bus and a functioning Secret Service are required. The pinned toolkit's DEB also declares WebKitGTK even though this UI never renders mail HTML; native GPU/runtime validation is separate from headless tests.

Download the matching `.sha256` or `SHA256SUMS` from the **same tagged release** and recompute before installation. No unsigned auto-updater is enabled.

Follow [SIGNING.md](../docs/SIGNING.md) to pin the certificate and verify manifest signature before package hashes. Linux is signed-manifest authenticated, not APT repository signing. No real-mailbox, GUI/device, long-term upgrade or store acceptance is implied by headless CI.
