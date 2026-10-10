# Dual-track self-signed release verification

Date: 2026-10-11, Asia/Shanghai. Target: imyemail-cloud-native 0.2.3 (native-v0.2.3) and imyemail-cloud-mygo 0.3.2 (mygo-v0.3.2), independent in logdns/imyemail-cloud. Original v0.2.0/v0.2.1 and mygo-v0.3.0 tags/assets are preserved; local chckemail-old is untouched. Historical package bytes, names, signatures and hashes are not rewritten.

## Source-first focused signing/release review

Scope: new signing scripts, release workflows, package finalization, public identity material, version continuity and active installation/documentation entry points. This is a focused delta review, not a new comprehensive client/protocol/upstream audit. Prior MyGo partial coverage remains limited to its historical source.

Controls: stable self-issued RSA-3072 CA:false/codeSigning certificate, separate stable Android key, no private material tracked; manual main-only tagged publication; tag/source equality and existing-release rejection; ephemeral hosted signing runners; scoped signing-job secrets; temporary Windows CurrentUser trust removed after signing; macOS temporary keychain/trust cleanup; manifest signed only after binary/package signing and hash regeneration. Third-party runtime signatures are not replaced. Inno-generated uninstallers are not separately signed.

Local gates: two isolated fixture tests check pinned PEM/DER/public-key equality, signed manifest acceptance, tamper rejection, PR signing denial and scratch-key cleanup; no real credentials/mailbox/network used. YAML and Bash syntax, shellcheck/actionlint, six-language localization and three Rust formatting checks are release gates. Remote regression/signing/public-download evidence is recorded after completion, not inferred from local syntax checks.

## Limits

Self-signed is not default OS trust, Developer ID/notarization, SmartScreen reputation or store approval. Linux uses signed checksum manifests, not APT repository signing. iOS remains unsigned and requires valid Apple provisioning. No real-mailbox/device/GPU GUI, long-term coexistence/upgrade/data restore, or off-machine signing-key backup acceptance is claimed. Android keys differ from old Chck/debug/private resigners; no cross-signer upgrade promise. Preserve separately backed-up databases and OS-vault credentials before updates.

## Edition naming and upgrade delta review

Native display/app/package prefixes change, not Bundle ID/applicationId/JNI/URL schemes, OS-vault namespaces, profile paths, Inno AppId/install directory or MSIX identity. New Linux DEB declares version-bounded Breaks/Replaces only against the legacy native package and provides a native command alias. Native Homebrew old/new casks are mutually exclusive; MyGo remains separate. No updater or cross-edition automatic migration is added. Windows signing now rejects a valid signature from an unexpected publisher on project binaries.

Screenshots use the real native UI and synthetic mail only, not edited mockups or real mailbox contents. Apple preview capture and MyGo headless renderer capture do not prove live receiving, GPU/window-manager behavior or feature parity.

Publication/download verification: pending until the final release workflows and independent downloaded-package checks complete.

## Pulse layout and initial remote regression

Source `9738a2e393ca31c469cea87ca4ffa45682745023`: native [CI 38068249840](https://github.com/logdns/imyemail-cloud/actions/runs/38068249840) passed 10/10; MyGo [CI 38068249845](https://github.com/logdns/imyemail-cloud/actions/runs/38068249845) passed 6/6. These runs predate the final Pulse layout and do not prove its final packages.

Latest MyGo delta: own Go components borrow Pulse's visual hierarchy only; no Pulse assets copied, no HTML/WebView or remote-content execution added. Read-only plain-text reader, mail/vault/RPC paths unchanged. Go vet, race suite and actual screenshot fixture pass in the same isolated no-network container. Light/dark at 880/1120 widths retain Inbox/Compose/account/read actions; unmatched filter state and loaded-cache count scope have regression tests. Three tap fixtures also pass: independent numeric version selection, separate cask identities, pinned-certificate manifest and package tamper rejection. README images show synthetic mail; native Preview and MyGo headless render are explicitly labeled. Screenshot hashes/provenance: [screenshots](../../docs/screenshots/README.md).

## Local gates passed

Final Pulse source `58b189c6e8cad92af9c4a33327c4e955e401ce17`: [native CI 38069687307](https://github.com/logdns/imyemail-cloud/actions/runs/38069687307) 10/10 and [MyGo CI 38069687465](https://github.com/logdns/imyemail-cloud/actions/runs/38069687465) 6/6 passed. First signing attempts [native 38070601083](https://github.com/logdns/imyemail-cloud/actions/runs/38070601083) / [MyGo 38070366865](https://github.com/logdns/imyemail-cloud/actions/runs/38070366865) were deliberately cancelled after macOS signing initialization hung following identity import. No public Release/assets were created. Their annotated tags native-v0.2.2/mygo-v0.3.1 remain unmoved; fixes use incremented versions. User-domain trust was interactive; hosted-only noninteractive admin-domain trust is now preflighted separately, with bounded setup/cleanup. Further preflight and final publication results are recorded below after verification.

Final naming/signing worktree copied to a dedicated Colima container with no network, read-only source/dependency/toolchain mounts, empty allowlisted environment, scratch HOME/TMP/caches, non-root UID, dropped capabilities/no-new-privileges, 2 CPUs/4 GiB memory, 256 PIDs, 1 GiB file limit, 6 GiB scratch, 900 CPU seconds and 1200-second wall limit. No host credentials, real mailbox or OS vault mounted. Exit 0: six edition identity/DEB replacement tests; two signing/tamper fixtures; Go vet and race tests; real MyGo renderer screenshot fixture; six-language localization consistency (643 × 6); core/Linux/push-bridge Rust formatting. Host-side actionlint, shellcheck for affected Bash scripts and git diff --check pass. Remote platform compilation/signing and native screenshot remain separate gates.
