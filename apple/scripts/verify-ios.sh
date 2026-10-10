#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
command -v xcodegen >/dev/null || { echo 'Install XcodeGen: brew install xcodegen' >&2; exit 1; }
RUN="$ROOT/.build-ios-results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN"
./scripts/check-ios-environment.sh > "$RUN/environment.txt"
python3 scripts/setup-ios-simulators.py > "$RUN/devices.json"
xcodegen generate --spec ios/project.yml
swift test -Xswiftc -warnings-as-errors > "$RUN/swift-tests.log" 2>&1
for family in iphone ipad; do
  udid="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' "$RUN/devices.json" "$family")"
  xcodebuild -project ios/ChckMailIOS.xcodeproj -scheme ChckMailIOSApp \
    -destination "platform=iOS Simulator,id=$udid" \
    -parallel-testing-enabled NO -collect-test-diagnostics never -derivedDataPath .build-ios-app \
    -resultBundlePath "$RUN/$family.xcresult" test CODE_SIGNING_ALLOWED=NO \
    IMYEMAIL_CLOUD_TEST_CONVERTER_URL="${IMYEMAIL_CLOUD_TEST_CONVERTER_URL:-}" \
    > "$RUN/$family.log" 2>&1
  xcrun xcresulttool get test-results summary --path "$RUN/$family.xcresult" > "$RUN/$family-summary.json"
done
./scripts/package-ios-simulator.sh > "$RUN/package.log" 2>&1
for family in iphone ipad; do
  udid="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' "$RUN/devices.json" "$family")"
  xcrun simctl install "$udid" "$ROOT/dist/imyemail-cloud-native Simulator.app"
  SIMCTL_CHILD_IMYEMAIL_CLOUD_PREVIEW=0 xcrun simctl launch --terminate-running-process "$udid" email.imy.cloud -chck.notify NO
done
printf 'iOS verification reports: %s\n' "$RUN"
