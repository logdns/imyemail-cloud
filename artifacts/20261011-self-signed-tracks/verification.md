# Dual-track self-signed release verification

Date: 2026-10-11, Asia/Shanghai. imyemail-cloud-native 0.2.2 (native-v0.2.2) and imyemail-cloud-mygo 0.3.1 (mygo-v0.3.1) remain independent in logdns/imyemail-cloud. Original v0.2.0/v0.2.1 and mygo-v0.3.0 tags/assets are preserved; local chckemail-old is untouched. Historical package bytes, names, signatures and hashes are not rewritten.

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

## Local gates passed

Final naming/signing worktree copied to a dedicated Colima container with no network, read-only source/dependency/toolchain mounts, empty allowlisted environment, scratch HOME/TMP/caches, non-root UID, dropped capabilities/no-new-privileges, 2 CPUs/4 GiB memory, 256 PIDs, 1 GiB file limit, 6 GiB scratch, 900 CPU seconds and 1200-second wall limit. No host credentials, real mailbox or OS vault mounted. Exit 0: six edition identity/DEB replacement tests; two signing/tamper fixtures; Go vet and race tests; real MyGo renderer screenshot fixture; six-language localization consistency (643 × 6); core/Linux/push-bridge Rust formatting. Host-side actionlint, shellcheck for affected Bash scripts and git diff --check pass. Remote platform compilation/signing and native screenshot remain separate gates.
