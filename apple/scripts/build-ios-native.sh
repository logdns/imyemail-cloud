#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE="${IMYEMAIL_CLOUD_CORE_ROOT:-$ROOT/../core}"
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
CARGO_CACHE="${CARGO_HOME:-$HOME/.cargo}"
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--remap-path-prefix=$CORE=/src/core --remap-path-prefix=$CARGO_CACHE=/cargo"
PLATFORM="${PLATFORM_NAME:-iphonesimulator}"
case "$PLATFORM" in
  iphonesimulator) DEFAULT_ARCH="$(uname -m)" ;;
  iphoneos) DEFAULT_ARCH=arm64 ;;
  *) echo "Unsupported platform: $PLATFORM" >&2; exit 1 ;;
esac
LIBRARIES=()
for arch in ${ARCHS:-$DEFAULT_ARCH}; do
  case "$PLATFORM/$arch" in
    iphonesimulator/arm64) target=aarch64-apple-ios-sim ;;
    iphonesimulator/x86_64) target=x86_64-apple-ios ;;
    iphoneos/arm64) target=aarch64-apple-ios ;;
    *) echo "Unsupported iOS architecture: $PLATFORM/$arch" >&2; exit 1 ;;
  esac
  IPHONEOS_DEPLOYMENT_TARGET=17.0 cargo build --manifest-path "$CORE/Cargo.toml" \
    -p chck-ffi-c --release --locked --target "$target"
  LIBRARIES+=("$CORE/target/$target/release/libchck_mail.a")
done
mkdir -p "$ROOT/Native/$PLATFORM"
xcrun lipo -create "${LIBRARIES[@]}" -output "$ROOT/Native/$PLATFORM/libchck_mail.a"
