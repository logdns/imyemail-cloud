#!/bin/sh
set -eu

APP_ID=email.imy.cloud
DESKTOP=/usr/share/applications/$APP_ID.desktop
METAINFO=/usr/share/metainfo/$APP_ID.metainfo.xml

for command_name in appstreamcli dbus-run-session desktop-file-validate gtk-launch pgrep timeout xvfb-run; do
  command -v "$command_name" >/dev/null 2>&1 || {
    echo "Missing desktop smoke-test tool: $command_name" >&2
    exit 2
  }
done

test -x /usr/bin/imyemail-cloud
test -f "$DESKTOP"
grep -q '^DBusActivatable=false$' "$DESKTOP"
desktop-file-validate "$DESKTOP"
appstreamcli validate --no-net "$METAINFO"

timeout 20s dbus-run-session -- xvfb-run -a sh -eu -c '
  gtk-launch "$1"
  pid=""
  attempts=0
  while [ "$attempts" -lt 50 ]; do
    pid=$(pgrep -n -x imyemail-cloud || true)
    [ -z "$pid" ] || break
    attempts=$((attempts + 1))
    sleep 0.2
  done
  [ -n "$pid" ] || {
    echo "Desktop launcher did not start imyemail-cloud" >&2
    exit 1
  }
  sleep 3
  kill -0 "$pid"
  kill "$pid"
' sh "$APP_ID"

printf 'desktop-launch-smoke: PASS\n'
