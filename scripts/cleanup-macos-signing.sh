#!/usr/bin/env bash
set -euo pipefail
test "${GITHUB_EVENT_NAME:-}" = workflow_dispatch
test "${GITHUB_REF:-}" = refs/heads/main
test "${RUNNER_ENVIRONMENT:-}" = github-hosted
directory="$RUNNER_TEMP/imyemail-signing"
security list-keychains -d user -s "$HOME/Library/Keychains/login.keychain-db"
policy="$directory/cleanup-trust-policy.plist"
original_policy=$(sudo -n security authorizationdb read com.apple.trust-settings.admin)
printf '%s\n' "$original_policy" > "$policy"
restore_policy() {
  cat "$policy" | sudo -n security authorizationdb write com.apple.trust-settings.admin
}
trap restore_policy EXIT
sudo -n security authorizationdb write com.apple.trust-settings.admin allow
sudo -n security remove-trusted-cert -d signing/release-cert.pem
restore_policy
trap - EXIT
security delete-keychain "$directory/release.keychain-db"
rm -f "$directory/release.p12" "$policy"
printf 'Removed temporary trust and private keychain; restored authorization policy\n'
