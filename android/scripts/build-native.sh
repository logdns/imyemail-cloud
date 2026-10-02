#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CORE="$ROOT/core"
ANDROID="$ROOT/android"
NDK="${ANDROID_NDK_HOME:-/opt/homebrew/share/android-ndk}"
API=26
HOST="$(uname -m)"
if [[ -d "$NDK/toolchains/llvm/prebuilt/darwin-arm64" ]]; then
  PREBUILT_DIR="darwin-arm64"
elif [[ -d "$NDK/toolchains/llvm/prebuilt/darwin-x86_64" ]]; then
  PREBUILT_DIR="darwin-x86_64"
elif [[ -d "$NDK/toolchains/llvm/prebuilt/linux-x86_64" ]]; then
  PREBUILT_DIR="linux-x86_64"
else
  echo "NDK llvm prebuilt not found under $NDK" >&2
  exit 1
fi
PREBUILT="$NDK/toolchains/llvm/prebuilt/$PREBUILT_DIR/bin"
echo "NDK host $HOST prebuilt $PREBUILT_DIR"
export ANDROID_NDK_HOME="$NDK"

copy_so() {
  local target="$1"
  local abi="$2"
  local clang="$3"
  export CC="$PREBUILT/$clang"
  export AR="$PREBUILT/llvm-ar"
  export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC"
  export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$CC"
  export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$CC"
  cargo build -p chck-ffi-c --release --target "$target" --manifest-path "$CORE/Cargo.toml"
  local dest="$ANDROID/core/chckcore/src/main/jniLibs/$abi"
  mkdir -p "$dest"
  cp "$CORE/target/$target/release/libchck_mail.so" "$dest/libchck_mail.so"
}

mkdir -p "$ANDROID/core/chckcore/src/main/jniLibs"
copy_so aarch64-linux-android arm64-v8a "aarch64-linux-android${API}-clang"
copy_so armv7-linux-androideabi armeabi-v7a "armv7a-linux-androideabi${API}-clang"
copy_so x86_64-linux-android x86_64 "x86_64-linux-android${API}-clang"
echo "jniLibs ready"
