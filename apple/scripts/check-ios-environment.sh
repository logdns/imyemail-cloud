#!/usr/bin/env bash
set -euo pipefail
sw_vers
uname -m
xcode-select -p
xcodebuild -version
xcodebuild -showsdks
swift --version
rustc --version
rustup target list --installed
xcrun simctl list runtimes
xcrun simctl list devices available
xcrun devicectl list devices
security find-identity -v -p codesigning
if command -v xcodegen >/dev/null; then xcodegen --version; fi
