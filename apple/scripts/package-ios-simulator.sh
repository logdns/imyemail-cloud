#!/usr/bin/env bash
set -euo pipefail
# Simulator development package. Device archives require an Apple signing team.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DERIVED="$ROOT/.build-ios-app"
APP="$ROOT/dist/imyemail-cloud-native Simulator.app"
cd "$ROOT"
command -v xcodegen >/dev/null || { echo 'Install XcodeGen: brew install xcodegen' >&2; exit 1; }
xcodegen generate --spec ios/project.yml
xcodebuild -project ios/ChckMailIOS.xcodeproj -scheme ChckMailIOSApp \
  -configuration Release -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath "$DERIVED" build ARCHS="$(uname -m)" CODE_SIGNING_ALLOWED=NO
PRODUCT="$DERIVED/Build/Products/Release-iphonesimulator/ChckMailIOS.app"
mkdir -p "$ROOT/dist"
STAGE="$(mktemp -d "$ROOT/dist/.ios-simulator.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
ditto "$PRODUCT" "$STAGE/imyemail-cloud-native Simulator.app"
codesign --force --sign - "$STAGE/imyemail-cloud-native Simulator.app"
codesign --verify --deep --strict "$STAGE/imyemail-cloud-native Simulator.app"
xcrun vtool -show-build "$STAGE/imyemail-cloud-native Simulator.app/ChckMailIOS" | grep -q 'platform IOSSIMULATOR'
rm -rf "$APP"
mv "$STAGE/imyemail-cloud-native Simulator.app" "$APP"
shasum -a 256 "$APP/ChckMailIOS" > "$ROOT/dist/imyemail-cloud-native-ios-simulator.sha256"
printf 'Simulator app (static Rust core): %s\n' "$APP"
