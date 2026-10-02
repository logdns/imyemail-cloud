#!/usr/bin/env bash
set -euo pipefail
SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
AVD="${IMYEMAIL_CLOUD_TEST_AVD:-chck_QA_Pixel_8_API_35}"
exec "$SDK/emulator/emulator" -avd "$AVD" -no-boot-anim -netdelay none -netspeed full "$@"
