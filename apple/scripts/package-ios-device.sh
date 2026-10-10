#!/usr/bin/env bash
set -euo pipefail

# Build an unsigned arm64 iPhone/iPad application and wrap it in an IPA.
# The result is signing input only; installation requires a valid Apple
# certificate and provisioning profile supplied outside this repository.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DERIVED="$ROOT/.build-ios-device"
IPA_NAME="${IMYEMAIL_CLOUD_IOS_IPA_NAME:-imyemail-cloud-native-ios-arm64-unsigned.ipa}"

[[ "$IPA_NAME" != */* && "$IPA_NAME" == *.ipa ]] || {
  echo 'invalid IMYEMAIL_CLOUD_IOS_IPA_NAME' >&2
  exit 1
}
command -v xcodegen >/dev/null || {
  echo 'Install XcodeGen: brew install xcodegen' >&2
  exit 1
}
rustup target list --installed | grep -Fx 'aarch64-apple-ios' >/dev/null || {
  echo 'Install the Rust iOS target: rustup target add aarch64-apple-ios' >&2
  exit 1
}

cd "$ROOT"
xcodegen generate --spec ios/project.yml
xcodebuild -project ios/ChckMailIOS.xcodeproj -scheme ChckMailIOSApp \
  -configuration Release -destination 'generic/platform=iOS' \
  -derivedDataPath "$DERIVED" -quiet build \
  ARCHS=arm64 ONLY_ACTIVE_ARCH=YES \
  CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO

PRODUCT="$DERIVED/Build/Products/Release-iphoneos/ChckMailIOS.app"
EXECUTABLE="$PRODUCT/ChckMailIOS"
test -d "$PRODUCT"
test -x "$EXECUTABLE"
test ! -e "$PRODUCT/embedded.mobileprovision"
test ! -e "$PRODUCT/_CodeSignature"
if codesign --verify --deep --strict "$PRODUCT" >/dev/null 2>&1; then
  echo 'expected an unsigned iOS application, but a valid signature was found' >&2
  exit 1
fi
file "$EXECUTABLE" | grep 'arm64' >/dev/null
xcrun vtool -show-build "$EXECUTABLE" | grep 'platform IOS' >/dev/null
nm -gU "$EXECUTABLE" | grep '_chck_mail_open' >/dev/null
[[ "$(plutil -extract CFBundleIdentifier raw "$PRODUCT/Info.plist")" == 'email.imy.cloud' ]]
if strings "$EXECUTABLE" | grep -E '/Users/[^/]+|BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|gh[pousr]_[A-Za-z0-9_]+' >/dev/null; then
  echo 'local developer path or secret marker found in iOS executable' >&2
  exit 1
fi

mkdir -p "$ROOT/dist"
STAGE="$(mktemp -d "$ROOT/dist/.ios-device.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/Payload"
ditto "$PRODUCT" "$STAGE/Payload/imyemail-cloud-native.app"
IPA="$ROOT/dist/$IPA_NAME"
rm -f "$IPA" "$IPA.sha256"
(cd "$STAGE" && COPYFILE_DISABLE=1 ditto -c -k --keepParent Payload "$IPA")
unzip -tq "$IPA"
unzip -Z1 "$IPA" | grep '^Payload/imyemail-cloud-native\.app/ChckMailIOS$' >/dev/null
if unzip -Z1 "$IPA" | grep -E '^__MACOSX/|\.dylib$' >/dev/null; then
  echo 'unexpected metadata or dynamic library found in IPA' >&2
  exit 1
fi
(cd "$ROOT/dist" && shasum -a 256 "$IPA_NAME" > "$IPA_NAME.sha256")

echo "unsigned IPA $IPA"
ls -lh "$IPA" "$IPA.sha256"
