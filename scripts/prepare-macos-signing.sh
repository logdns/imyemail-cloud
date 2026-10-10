#!/usr/bin/env bash
set -euo pipefail
test "${GITHUB_EVENT_NAME:-}" = workflow_dispatch
test "${GITHUB_REF:-}" = refs/heads/main
test "${RUNNER_ENVIRONMENT:-}" = github-hosted
umask 077
directory="$RUNNER_TEMP/imyemail-signing"
mkdir -p "$directory"
printf '%s' "$RELEASE_P12_BASE64" | base64 --decode > "$directory/release.p12"
password=$(openssl rand -hex 32)
keychain="$directory/release.keychain-db"
security create-keychain -p "$password" "$keychain"
security set-keychain-settings -lut 7200 "$keychain"
security unlock-keychain -p "$password" "$keychain"
security list-keychains -d user -s "$keychain" "$HOME/Library/Keychains/login.keychain-db"
security import "$directory/release.p12" -k "$keychain" -P "$RELEASE_P12_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple: -s -k "$password" "$keychain" >/dev/null
security add-trusted-cert -r trustRoot -p codeSign -k "$keychain" signing/release-cert.pem
identity=$(openssl x509 -in signing/release-cert.pem -noout -fingerprint -sha1 | cut -d= -f2 | tr -d ':')
security find-identity -v -p codesigning "$keychain" | grep -F "$identity"
printf 'IMYEMAIL_CLOUD_MACOS_SIGNING_IDENTITY=%s\nIMYEMAIL_CLOUD_MACOS_SIGNING_KEYCHAIN=%s\n' "$identity" "$keychain" >> "$GITHUB_ENV"
