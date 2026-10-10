# Install imyemail-cloud on Linux

[Current installation](https://github.com/logdns/imyemail-cloud/blob/main/docs/INSTALL.md) · [Signature verification](https://github.com/logdns/imyemail-cloud/blob/main/docs/SIGNING.md) · [Preserved versions](https://github.com/logdns/imyemail-cloud/blob/main/docs/VERSIONS.md)

Native 0.2.2: [native-v0.2.2](https://github.com/logdns/imyemail-cloud/releases/tag/native-v0.2.2), x86_64/ARM64 DEB. MyGo 0.3.1: [mygo-v0.3.1](https://github.com/logdns/imyemail-cloud/releases/tag/mygo-v0.3.1), amd64/arm64 DEB or portable tar.gz. Old releases remain available; do not mix packages/checksums across tags.

Verify the pinned RSA certificate, signed SHA256SUMS and package hash before `sudo apt install ./YOUR_PACKAGE.deb`. This is signed-manifest authentication, not APT repository signing; historical GPG instructions/keys do not apply to these releases.

Original uses GTK4/libadwaita/libsecret/WebKitGTK. MyGo uses a compatible Ubuntu 24.04 runtime, GTK 3, D-Bus and an unlocked Secret Service; retain bundled core/licenses. Install only the matching architecture. Apps/data remain independent.

Launch `imyemail-cloud-native` or `imyemail-cloud-mygo`. Remove only that package with `sudo apt remove imyemail-cloud-native` or `sudo apt remove imyemail-cloud-mygo`; do not purge/autoremove or delete mail/credentials. Back up database and OS vault before updates; follow version-specific rollback guidance.
