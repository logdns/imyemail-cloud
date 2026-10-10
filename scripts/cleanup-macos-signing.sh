#!/usr/bin/env bash
set -euo pipefail
test "${GITHUB_EVENT_NAME:-}" = workflow_dispatch
test "${GITHUB_REF:-}" = refs/heads/main
test "${RUNNER_ENVIRONMENT:-}" = github-hosted
directory="$RUNNER_TEMP/imyemail-signing"
security list-keychains -d user -s "$HOME/Library/Keychains/login.keychain-db"
fingerprint=$(openssl x509 -in signing/release-cert.pem -noout -fingerprint -sha256 | cut -d= -f2 | tr -d ':')
test "$fingerprint" = 476EA6E573714E9C46A43471B5938D791E1C3851BEB7B3EEF8269CF084F48548
sudo -n security delete-certificate -Z "$fingerprint" /Library/Keychains/System.keychain
security delete-keychain "$directory/release.keychain-db"
rm -f "$directory/release.p12"
printf 'Removed matched system certificate and private keychain; hosted runner teardown clears its trust-settings record\n'
