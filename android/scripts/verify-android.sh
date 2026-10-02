#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ -z "${JAVA_HOME:-}" && -d /opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home ]]; then
    export JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home
fi
SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
TOOLS="${ANDROID_BUILD_TOOLS:-$SDK/build-tools/36.0.0}"
./scripts/build-native.sh
./gradlew :core:common:test :core:data:test :core:chckcore:testDebugUnitTest :app:assembleDebug :app:lintDebug --console=plain
APK=app/build/outputs/apk/debug/app-debug.apk
"$TOOLS/apksigner" verify --verbose "$APK"
"$TOOLS/zipalign" -c -P 16 4 "$APK"
python3 - "$APK" <<'PY'
import hashlib
from pathlib import Path
import sys
import zipfile
apk = Path(sys.argv[1])
with zipfile.ZipFile(apk) as package:
    for abi in ('arm64-v8a', 'armeabi-v7a', 'x86_64'):
        bundled = package.read(f'lib/{abi}/libchck_mail.so')
        built = Path(f'core/chckcore/src/main/jniLibs/{abi}/libchck_mail.so').read_bytes()
        if bundled != built:
            raise SystemExit(f'{abi}: packaged library differs from this build')
        print(abi, hashlib.sha256(bundled).hexdigest())
print(apk, hashlib.sha256(apk.read_bytes()).hexdigest())
PY
printf 'Debug APK verified. Device installation and mail delivery need separate validation.\n'
