#!/usr/bin/env bash
set -euo pipefail
directory=${1:?release asset directory required}
test "${GITHUB_EVENT_NAME:-}" = workflow_dispatch
test "${GITHUB_REF:-}" = refs/heads/main
umask 077
key=$(mktemp "$RUNNER_TEMP/release-key.XXXXXX")
trap 'rm -f "$key"' EXIT
printf '%s' "$RELEASE_KEY_BASE64" | base64 --decode > "$key"
cp signing/release-cert.pem signing/release-cert.der signing/release-public.pem "$directory/"
printf 'source_sha=%s\nworkflow_run=%s/actions/runs/%s\n' "$GITHUB_SHA" "https://github.com/$GITHUB_REPOSITORY" "$GITHUB_RUN_ID" > "$directory/provenance.txt"
python3 - "$directory" <<'PY'
import hashlib
import pathlib
import sys
root = pathlib.Path(sys.argv[1])
for path in sorted(root.iterdir()):
    if path.is_file() and path.suffix in {".zip", ".exe", ".apk", ".ipa", ".deb", ".gz"}:
        digest = hashlib.file_digest(path.open("rb"), "sha256").hexdigest()
        path.with_name(path.name + ".sha256").write_text(digest + "  " + path.name + "\n", encoding="utf-8")
files = sorted(path for path in root.iterdir() if path.is_file() and path.name not in {"SHA256SUMS", "SHA256SUMS.sig"})
with (root / "SHA256SUMS").open("w", encoding="utf-8") as output:
    for path in files:
        output.write(hashlib.file_digest(path.open("rb"), "sha256").hexdigest() + "  " + path.name + "\n")
PY
openssl dgst -sha256 -sign "$key" -out "$directory/SHA256SUMS.sig" "$directory/SHA256SUMS"
openssl dgst -sha256 -verify signing/release-public.pem -signature "$directory/SHA256SUMS.sig" "$directory/SHA256SUMS"
