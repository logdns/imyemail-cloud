# Independent editions / 双版本开发与升级

| Track | Current | Retained history | Identity |
| --- | --- | --- | --- |
| `imyemail-cloud-native` | `native-v0.2.4` | `v0.2.0`, `v0.2.1` | `email.imy.cloud`, existing platform stores |
| `imyemail-cloud-mygo` desktop preview | `mygo-v0.3.3` | `mygo-v0.3.0` | `email.imy.cloud.mygo`, separate config |

Both remain in [logdns/imyemail-cloud](https://github.com/logdns/imyemail-cloud). Original `apple/`, `android/`, `linux/`, `windows/`, shared `core/` and services remain maintained; `desktop-mygo/` is additive. No historical public tag/package is replaced. Homebrew uses separate tokens. Dated evidence stays historical, not rewritten to claim a new signer.

原始 Chck 工作树、资料与全部历史仍在本地 `chckemail-old/` 和其中的 `chckemail-original.bundle`，被 Git 忽略，不公开旧商业资料、签名或私人文件。公开原版通过不可移动标签和 Release 保留，无需重复复制进新源码目录。

## Restore historical source without changing main

```bash
git fetch origin --tags
git worktree add --detach ../imyemail-cloud-v0.2.1 v0.2.1
git worktree add --detach ../imyemail-cloud-mygo-v0.3.0 mygo-v0.3.0
```

These are separate detached inspection/build worktrees. Deliberately branch from the selected track for future development; never move published tags or use old packages to prove new code. Historical toolchains/SDKs may be required.

## Future releases

Native: `native-vMAJOR.MINOR.PATCH`, `release-build.yml`; MyGo: `mygo-vMAJOR.MINOR.PATCH`, `mygo-desktop.yml`. Historical `v*` tags remain native legacy history, not renamed aliases. Shared core changes require both regression suites. Version manifests/lockfiles agree; Android versionCode increases. Run CI, create an annotated tag at verified source, manually dispatch publication from unchanged main. Workflows reject tag mismatch/existing releases. Native Latest and explicit MyGo entry stay separate.

| Ownership | Native | MyGo |
| --- | --- | --- |
| UI source | `apple/`, `android/`, `linux/`, `windows/` | `desktop-mygo/` |
| Package prefix | `imyemail-cloud-native-` | `imyemail-cloud-mygo-` |
| macOS app | `imyemail-cloud-native.app` | `imyemail-cloud-mygo.app` |
| Homebrew cask | `imyemail-cloud-native` | `imyemail-cloud-mygo` |
| Linux DEB package | `imyemail-cloud-native` | `imyemail-cloud-mygo` |
| CI | `ci.yml` | `mygo-desktop.yml` |
| Version ownership | Rust workspace, platform metadata and lockfiles | `main.go`, `mygo.json`; shared core stays independently versioned |

## Rename without losing upgrades / 改名不改账号身份

Native display/package names change, not its Bundle ID, Android applicationId, JNI symbols, URI schemes, Windows MSIX identity, Inno AppId/install directory, OS-vault namespace or database paths. The shared Rust CLI and Windows executable retain `imyemail-cloud` internally for existing integrations. The native cask exposes that CLI as `imyemail-cloud-native`, while MyGo bundles `imyemail-cloud-core`; they do not collide.

Linux native DEB declares version-bounded `Breaks/Replaces: imyemail-cloud (<< 0.2.4)` and retains `/usr/bin/imyemail-cloud` with a new `imyemail-cloud-native` alias. APT may replace the legacy package, never MyGo; inspect its proposed transaction and cancel if unrelated packages would be removed. Existing desktop ID/profile/vault are unchanged.

Tags `native-v0.2.2` and `mygo-v0.3.1` retain a cancelled signing attempt at source `58b189c`; no downloadable Release was published for those tags. A hosted macOS trust prompt blocked signing. The fix uses incremented versions, not moved tags or replaced packages.

Tags `native-v0.2.3` and `mygo-v0.3.2` likewise preserve a cancelled attempt at `c094a06`: macOS/Linux/Android/iOS packages built, but Windows user-root certificate import waited for interactive consent. No Release was published. Final hosted Windows preflight uses admin LocalMachine public trust and removes that trust after signing; client installation does not change machine-wide trust.

macOS old and new native app bundles have the same ID and share data: **do not run both**. Close the old native app, back up its database and Keychain credentials, then replace only the old application bundle through the matching package manager. Do not rename/delete its profile. Homebrew's old `imyemail-cloud` token is preserved as a legacy cask; migrate explicitly with cleanup disabled, then install `imyemail-cloud-native`. Both casks conflict with each other, neither conflicts with MyGo. No automatic cross-edition account migration is provided.

历史包的名称、签名、哈希和标签不改；新下载、界面与维护命令统一使用上述双版本名称。旧原生内部文件名是兼容边界，不是第三个产品。修复 UI 只发对应版本；共享核心修复必须两版回归，随后分别发递增补丁，禁止用另一版覆盖更新。

Preserve fixed keys per [SIGNING.md](SIGNING.md). Never delete the other edition or mail/credentials as an update step. Update bilingual installation, root/platform READMEs, release notes and corresponding cask using verified names/hashes.

## Rollback

Back up each edition's database **and OS vault** before upgrading. Use a historical package with its matching manifest/signature and compatible data backup; downgrade/schema and real-device upgrades are not guaranteed by unit tests. Android normally rejects lower versionCode; prefer forward fixes. No destructive purge/cleanup, public mailbox backups, tag movement or silent asset replacement.
