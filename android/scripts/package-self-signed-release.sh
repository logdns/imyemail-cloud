#!/usr/bin/env bash
set -Eeuo pipefail
cd "$(dirname "$0")/.."

if [[ -z "${JAVA_HOME:-}" && -d /opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home ]]; then
  export JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home
fi
SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
TOOLS="${ANDROID_BUILD_TOOLS:-$SDK/build-tools/36.0.0}"
KEYSTORE="${IMYEMAIL_CLOUD_ANDROID_KEYSTORE:?set IMYEMAIL_CLOUD_ANDROID_KEYSTORE}"
ALIAS="${IMYEMAIL_CLOUD_ANDROID_KEY_ALIAS:?set IMYEMAIL_CLOUD_ANDROID_KEY_ALIAS}"
PASSWORD_FILE="${IMYEMAIL_CLOUD_ANDROID_KEY_PASSWORD_FILE:?set IMYEMAIL_CLOUD_ANDROID_KEY_PASSWORD_FILE}"
RELEASE_DATE="${IMYEMAIL_CLOUD_RELEASE_DATE:-$(date +%Y%m%d)}"
[[ -f "$KEYSTORE" && -s "$PASSWORD_FILE" ]] || { echo 'Android signing material is missing' >&2; exit 1; }
[[ "$RELEASE_DATE" =~ ^[0-9]{8}$ ]] || { echo 'invalid IMYEMAIL_CLOUD_RELEASE_DATE' >&2; exit 1; }
SIGNING_PASSWORD=$(<"$PASSWORD_FILE")
[[ -n "$SIGNING_PASSWORD" ]] || { echo 'Android signing password is empty' >&2; exit 1; }
export SIGNING_PASSWORD

./scripts/build-native.sh
./gradlew :core:common:test :core:data:test :core:chckcore:testDebugUnitTest \
  :app:assembleRelease :app:lintRelease --console=plain

UNSIGNED=app/build/outputs/apk/release/app-release-unsigned.apk
ALIGNED=app/build/outputs/apk/release/app-release-aligned.apk
OUTPUT="dist/imyemail-cloud-android-$RELEASE_DATE-release-selfsigned.apk"
mkdir -p dist
"$TOOLS/zipalign" -f -P 16 4 "$UNSIGNED" "$ALIGNED"
"$TOOLS/apksigner" sign \
  --ks "$KEYSTORE" --ks-key-alias "$ALIAS" \
  --ks-pass env:SIGNING_PASSWORD --key-pass env:SIGNING_PASSWORD \
  --out "$OUTPUT" "$ALIGNED"
"$TOOLS/apksigner" verify --verbose --print-certs "$OUTPUT"
"$TOOLS/zipalign" -c -P 16 4 "$OUTPUT"

python3 - "$OUTPUT" <<'PY'
import hashlib
from pathlib import Path
import sys
import zipfile

apk = Path(sys.argv[1])
with zipfile.ZipFile(apk) as package:
    for abi in ("arm64-v8a", "armeabi-v7a", "x86_64"):
        bundled = package.read(f"lib/{abi}/libchck_mail.so")
        built = Path(f"core/chckcore/src/main/jniLibs/{abi}/libchck_mail.so").read_bytes()
        if bundled != built:
            raise SystemExit(f"{abi}: packaged library differs from this build")
        print(abi, hashlib.sha256(bundled).hexdigest())
print(apk, hashlib.sha256(apk.read_bytes()).hexdigest())
PY

(cd "$(dirname "$OUTPUT")" && shasum -a 256 "$(basename "$OUTPUT")") > "$OUTPUT.sha256"
printf 'release_apk=%s\n' "$OUTPUT"
