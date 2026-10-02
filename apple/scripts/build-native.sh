#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CORE="$ROOT/core"
APPLE="$ROOT/apple"
OUT="$APPLE/dist/native"
mkdir -p "$OUT"

cargo build -p chck-ffi-c --release --manifest-path "$CORE/Cargo.toml"
mkdir -p "$APPLE/Native/macos" "$APPLE/Sources/ChckAppCore/Resources"
cp "$CORE/target/release/libchck_mail.dylib" "$OUT/libchck_mail.dylib"
cp "$CORE/target/release/libchck_mail.dylib" "$APPLE/Native/macos/libchck_mail.dylib"
cp "$CORE/target/release/libchck_mail.dylib" "$APPLE/Sources/ChckAppCore/Resources/libchck_mail.dylib"

build_ios() {
  local target="$1"
  cargo build -p chck-ffi-c --release --target "$target" --manifest-path "$CORE/Cargo.toml"
  mkdir -p "$OUT/$target"
  cp "$CORE/target/$target/release/libchck_mail.a" "$OUT/$target/libchck_mail.a"
  if [[ "$target" == "aarch64-apple-ios" ]]; then
    mkdir -p "$APPLE/Native/ios"
    cp "$CORE/target/$target/release/libchck_mail.a" "$APPLE/Native/ios/libchck_mail.a"
  fi
  if [[ -f "$CORE/target/$target/release/libchck_mail.dylib" ]]; then
    mkdir -p "$APPLE/Native/ios-sim"
    cp "$CORE/target/$target/release/libchck_mail.dylib" "$APPLE/Native/ios-sim/libchck_mail.dylib"
  fi
  if [[ "$target" == "aarch64-apple-ios-sim" ]]; then
    mkdir -p "$APPLE/Native/ios-sim"
    cp "$CORE/target/$target/release/libchck_mail.a" "$APPLE/Native/ios-sim/libchck_mail.a"
  fi
}

if command -v rustup >/dev/null && rustup target list --installed | grep -q '^aarch64-apple-ios$'; then
  IPHONEOS_DEPLOYMENT_TARGET=17.0 build_ios aarch64-apple-ios
fi
if command -v rustup >/dev/null && rustup target list --installed | grep -q '^aarch64-apple-ios-sim$'; then
  IPHONEOS_DEPLOYMENT_TARGET=17.0 build_ios aarch64-apple-ios-sim
fi
echo "native libs in $OUT"
