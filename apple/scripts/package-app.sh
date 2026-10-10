#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
# Build and stage the same core version before compiling/bundling the UI.
CORE_ROOT="${IMYEMAIL_CLOUD_CORE_ROOT:-$ROOT/../core}"
CARGO_CACHE="${CARGO_HOME:-$HOME/.cargo}"
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--remap-path-prefix=$CORE_ROOT=/src/core --remap-path-prefix=$CARGO_CACHE=/cargo"
cargo build --manifest-path "$CORE_ROOT/Cargo.toml" --release -p chck-cli -p chck-ffi-c
mkdir -p "$ROOT/Native/macos" "$ROOT/Sources/ChckAppCore/Resources"
cp "$CORE_ROOT/target/release/libchck_mail.dylib" "$ROOT/Native/macos/"
cp "$CORE_ROOT/target/release/libchck_mail.dylib" "$ROOT/Sources/ChckAppCore/Resources/"
swift build -c release --product ChckMailMac
BIN_DIR="$(swift build -c release --show-bin-path)"
BIN="$BIN_DIR/ChckMailMac"
APP="$ROOT/dist/imyemail-cloud-native.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
cp "$BIN" "$APP/Contents/MacOS/ChckMail"
cp "$ROOT/packaging/Info.plist" "$APP/Contents/Info.plist"
cp "$ROOT/packaging/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
cp "$ROOT/Sources/ChckMailMac/Resources/PrivacyInfo.xcprivacy" "$APP/Contents/Resources/"
cp "$ROOT/Sources/ChckDesign/Resources/Media.xcassets/BrandMark.imageset/BrandMark.png" "$APP/Contents/Resources/BrandMark.png"
for bundle in ChckMail_ChckDesign.bundle ChckMail_ChckAppCore.bundle ChckMail_ChckMailMac.bundle; do
  if [[ -d "$BIN_DIR/$bundle" ]]; then
    cp -R "$BIN_DIR/$bundle" "$APP/Contents/Resources/"
  fi
done
CORE_DEBUG="$CORE_ROOT/target/debug/imyemail-cloud"
CORE_RELEASE="$CORE_ROOT/target/release/imyemail-cloud"
if [[ -x "$CORE_RELEASE" ]]; then
  cp "$CORE_RELEASE" "$APP/Contents/MacOS/imyemail-cloud"
elif [[ -x "$CORE_DEBUG" ]]; then
  cp "$CORE_DEBUG" "$APP/Contents/MacOS/imyemail-cloud"
fi
LIB_RELEASE="$CORE_ROOT/target/release/libchck_mail.dylib"
LIB_DEBUG="$CORE_ROOT/target/debug/libchck_mail.dylib"
LIB_NATIVE="$ROOT/Native/macos/libchck_mail.dylib"
LIB_SRC=""
if [[ -f "$LIB_RELEASE" ]]; then
  LIB_SRC="$LIB_RELEASE"
elif [[ -f "$LIB_DEBUG" ]]; then
  LIB_SRC="$LIB_DEBUG"
elif [[ -f "$LIB_NATIVE" ]]; then
  LIB_SRC="$LIB_NATIVE"
fi
if [[ -n "$LIB_SRC" ]]; then
  cp "$LIB_SRC" "$APP/Contents/Frameworks/libchck_mail.dylib"
  install_name_tool -id "@executable_path/../Frameworks/libchck_mail.dylib" \
    "$APP/Contents/Frameworks/libchck_mail.dylib"
fi
SIGN_IDENTITY="${IMYEMAIL_CLOUD_MACOS_SIGNING_IDENTITY:--}"
SIGN_ARGS=(--force --sign "$SIGN_IDENTITY")
if [[ -n "${IMYEMAIL_CLOUD_MACOS_SIGNING_KEYCHAIN:-}" ]]; then
  SIGN_ARGS+=(--keychain "$IMYEMAIL_CLOUD_MACOS_SIGNING_KEYCHAIN")
fi
if [[ "$SIGN_IDENTITY" != "-" ]]; then
  SIGN_ARGS+=(--options runtime --timestamp=none)
fi
while IFS= read -r -d '' CANDIDATE; do
  if file "$CANDIDATE" | grep -q 'Mach-O'; then
    /usr/bin/codesign "${SIGN_ARGS[@]}" "$CANDIDATE"
  fi
done < <(find "$APP/Contents" -type f -print0)
/usr/bin/codesign "${SIGN_ARGS[@]}" "$APP"
/usr/bin/codesign --verify --deep --strict "$APP"
ARCH="$(uname -m)"
ZIP_NAME="${IMYEMAIL_CLOUD_MACOS_ZIP_NAME:-imyemail-cloud-native-macos-$ARCH.zip}"
[[ "$ZIP_NAME" != */* && "$ZIP_NAME" == *.zip ]] || { echo 'invalid IMYEMAIL_CLOUD_MACOS_ZIP_NAME' >&2; exit 1; }
ZIP="$ROOT/dist/$ZIP_NAME"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"
(cd "$(dirname "$ZIP")" && shasum -a 256 "$(basename "$ZIP")") > "$ZIP.sha256"
echo "built $APP"
echo "zip $ZIP"
ls -lh "$APP/Contents/MacOS/ChckMail" "$APP/Contents/Frameworks/libchck_mail.dylib" "$ZIP"
