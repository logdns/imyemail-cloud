#!/bin/sh
set -eu

PACKAGE=${1:?usage: verify-deb.sh path/to/package.deb}
EXPECTED_ARCH=${2:-}

test -f "$PACKAGE"
ACTUAL_ARCH=$(dpkg-deb -f "$PACKAGE" Architecture)
if [ -n "$EXPECTED_ARCH" ] && [ "$ACTUAL_ARCH" != "$EXPECTED_ARCH" ]; then
  echo "Expected architecture $EXPECTED_ARCH, got $ACTUAL_ARCH" >&2
  exit 1
fi

CONTENTS=$(dpkg-deb -c "$PACKAGE")
for path in \
  ./usr/bin/imyemail-cloud \
  ./usr/share/applications/email.imy.cloud.desktop \
  ./usr/share/metainfo/email.imy.cloud.metainfo.xml \
  ./usr/share/icons/hicolor/scalable/apps/email.imy.cloud.svg \
  ./usr/share/icons/hicolor/128x128/apps/email.imy.cloud.png \
  ./usr/share/icons/hicolor/512x512/apps/email.imy.cloud.png \
  ./usr/share/doc/imyemail-cloud/README.md \
  ./usr/share/doc/imyemail-cloud/README.zh-CN.md; do
  printf '%s\n' "$CONTENTS" | grep -q " $path$" || {
    echo "Package is missing $path" >&2
    exit 1
  }
done

TMP=$(mktemp -d "${TMPDIR:-/tmp}/linux-deb-verify.XXXXXX")
cleanup() {
  find "$TMP" -depth -delete 2>/dev/null || true
}
trap cleanup EXIT HUP INT TERM
dpkg-deb -x "$PACKAGE" "$TMP"

grep -q '^Exec=imyemail-cloud %u$' "$TMP/usr/share/applications/email.imy.cloud.desktop"
grep -q '^Icon=email.imy.cloud$' "$TMP/usr/share/applications/email.imy.cloud.desktop"
grep -q '^DBusActivatable=false$' "$TMP/usr/share/applications/email.imy.cloud.desktop"
dpkg-deb -f "$PACKAGE" Depends | grep -q 'libc6 (>= 2.34)'
grep -q 'https://imy.email' "$TMP/usr/share/metainfo/email.imy.cloud.metainfo.xml"
file "$TMP/usr/bin/imyemail-cloud"
printf 'deb-package-layout: PASS (%s)\n' "$ACTUAL_ARCH"
