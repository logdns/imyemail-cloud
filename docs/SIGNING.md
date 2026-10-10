# Self-signed releases / 自签校验

[Install / 安装](INSTALL.zh-CN.md) · [Release policy](RELEASE.md) · [Preserved versions](VERSIONS.md)

Stable release identity: self-issued RSA-3072 code-signing leaf, CA:false, valid 2026-10-10 to 2031-10-09 UTC. Used for both desktop editions and RSA/SHA-256 manifests. Only public material is tracked in `signing/`; private keys stay outside Git and in manually dispatched main-branch release/preflight secrets on ephemeral hosted runners. PR/development jobs receive no signing secrets. Self-signed is not public trust, Apple Developer ID/notarization, SmartScreen reputation or store approval.

完整代码签名证书 SHA-256 / complete certificate fingerprint:

```text
476ea6e573714e9c46a43471b5938d791e1c3851beb7b3eef8269cf084f48548
```

Separate stable Android app-signing certificate SHA-256:

```text
bf5abd221fb25fa384223e2be1543ddf2818998c5ae4d1f3175a73f9faa9d10d
```

## Signed manifest (OpenSSL)

Download package, manifest/signature/certificate from the same release. Compare the certificate fingerprint with a **previously trusted** copy of this source/document, not just another file on the same download channel.

```bash
openssl x509 -in release-cert.pem -noout -fingerprint -sha256
openssl x509 -in release-cert.pem -pubkey -noout > verified-public.pem
openssl dgst -sha256 -verify verified-public.pem -signature SHA256SUMS.sig SHA256SUMS
grep -F '  YOUR_PACKAGE_NAME' SHA256SUMS
shasum -a 256 YOUR_PACKAGE_NAME
# Linux: sha256sum YOUR_PACKAGE_NAME
```

After `Verified OK`, compare the exact filename and entire digest. `sha256sum -c SHA256SUMS` checks all assets and reports missing files for partial downloads; subset `.sha256` checks are valid only after confirming against the **signed manifest**. Hashes alone do not authenticate a publisher. Linux DEB/tar authentication is manifest-based, not APT repository/embedded DEB signing.

## macOS extracted bundle

```bash
codesign --verify --deep --strict /Applications/imyemail-cloud-mygo.app
codesign -d --extract-certificates=/tmp/imyemail-leaf /Applications/imyemail-cloud-mygo.app
openssl x509 -inform DER -in /tmp/imyemail-leaf0 -noout -fingerprint -sha256
```

Use the original app name for that edition. Leaf fingerprint must match. Chain trust/Gatekeeper can still reject signed bytes: use normal OS approval, never disable Gatekeeper/SIP/quarantine protection. Not notarized; no certificate is automatically trusted on your host.

Hosted signing uses a disposable private-key keychain and admin-domain trust to avoid GUI prompts. Cleanup deletes the fingerprint-matched system certificate, private keychain and temporary P12; runner teardown clears its orphaned trust-settings record. User-domain trust removal is not used because it can wait indefinitely for interactive authorization. These scripts reject self-hosted runners and are not user installation commands. The bounded preflight signs/verifies a dummy binary, compares the extracted certificate fingerprint and checks cleanup without publishing it.

## Windows Authenticode / 可选用户信任

PowerShell 7+:

```powershell
$Certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new((Resolve-Path .\release-cert.der).Path)
$Fingerprint = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($Certificate.RawData)).ToLowerInvariant()
if ($Fingerprint -ne '476ea6e573714e9c46a43471b5938d791e1c3851beb7b3eef8269cf084f48548') { throw 'Unexpected certificate' }
Get-FileHash .\YOUR_PACKAGE -Algorithm SHA256
$Signature = Get-AuthenticodeSignature .\imyemail-cloud-mygo.exe
$Signature | Format-List Status,StatusMessage,SignerCertificate
if ($Signature.SignerCertificate.Thumbprint -ne $Certificate.Thumbprint) { throw 'Unexpected signer' }
```

Original installer/application and MyGo UI/core use the release signer; third-party runtimes retain upstream signatures. ZIPs have no Authenticode signature: check both extracted executables and the signed archive manifest. Verify manifests using separately trusted OpenSSL (above). Self-signed chains are untrusted by default.

Only if you intentionally trust this fingerprint-verified publisher, optionally import for **your own user**:

```powershell
Import-Certificate -FilePath .\release-cert.der -CertStoreLocation Cert:\CurrentUser\Root
Import-Certificate -FilePath .\release-cert.der -CertStoreLocation Cert:\CurrentUser\TrustedPublisher
```

This trusts this code-signing key for that user, not just one file; not a SmartScreen bypass. Never import mismatched certificates or LocalMachine/global roots. Remove optional trust:

```powershell
$Thumbprint = $Certificate.Thumbprint
Remove-Item "Cert:\CurrentUser\Root\$Thumbprint" -ErrorAction SilentlyContinue
Remove-Item "Cert:\CurrentUser\TrustedPublisher\$Thumbprint" -ErrorAction SilentlyContinue
```

Signatures have no trusted timestamp; certificate expiry/rotation requires forward releases and updated fingerprints. Check status/chain again after trust changes. Signing tests do not prove GUI install/upgrade/device acceptance.

## Android / iOS

Use SDK `apksigner verify --verbose --print-certs YOUR.apk` and compare the Android SHA-256 above. Future Android updates retain app ID, this key and increasing versionCode. This imyemail identity is not the archived Chck/debug key; cross-signer upgrades are unsupported. Ordinary self-signing cannot supply iOS Apple provisioning.

## Key preservation / 密钥保留

Maintainer identities are in protected `.imyemail-cloud-signing/` **outside Git**, directory 0700/private files 0600. Encrypted P12s and separate passwords are retained for future releases; secrets enter only manually dispatched main-branch ephemeral hosted signing jobs. No per-build key regeneration. Keep encrypted offline backups of both identities/passwords before host/CI changes; no off-machine backup is claimed here.

Never publish private keys/passwords. If compromised/lost, document rotation in a new release; Android requires supported signing lineage or deliberate new app identity, not silent key replacement. Preserve historical public certificates/manifests for old-package verification.
