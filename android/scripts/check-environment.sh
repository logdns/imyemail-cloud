#!/usr/bin/env bash
# Read-only inventory, except adb may start its local daemon.
set -uo pipefail
SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
JDK="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home}"
NDK="${ANDROID_NDK_HOME:-/opt/homebrew/share/android-ndk}"
printf 'Host: '; uname -sm
printf 'JDK: %s\nSDK: %s\nNDK: %s\n' "$JDK" "$SDK" "$NDK"
if [[ -x "$JDK/bin/java" ]]; then "$JDK/bin/java" -version; else java -version; fi
if [[ -f "$NDK/source.properties" ]]; then cat "$NDK/source.properties"; fi
rustup target list --installed
if [[ -d "$SDK/platforms" ]]; then ls "$SDK/platforms"; fi
if [[ -d "$SDK/build-tools" ]]; then ls "$SDK/build-tools"; fi
if [[ -x "$SDK/platform-tools/adb" ]]; then "$SDK/platform-tools/adb" devices -l; else echo 'adb unavailable'; fi
if [[ -x "$SDK/emulator/emulator" ]]; then
    printf 'AVDs:\n'
    "$SDK/emulator/emulator" -list-avds
else echo 'emulator unavailable'; fi
if command -v docker >/dev/null; then docker ps --format '{{.Names}} {{.Status}}'; else echo 'Docker unavailable'; fi
# Missing runtime devices/services are reported above, independently of build readiness.
