#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
VERSION=${IMYEMAIL_CLOUD_LINUX_VERSION:-$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)}
BUILD_DATE=${IMYEMAIL_CLOUD_LINUX_BUILD_DATE:-$(date -u +%Y%m%d)}
MACHINE=${IMYEMAIL_CLOUD_LINUX_MACHINE:-$(uname -m)}

case "$MACHINE" in
  aarch64|arm64)
    DEB_ARCH=arm64
    FILE_ARCH=arm64
    ;;
  x86_64|amd64)
    DEB_ARCH=amd64
    FILE_ARCH=x86_64
    ;;
  *)
    echo "Unsupported Linux architecture: $MACHINE" >&2
    exit 2
    ;;
esac

for command_name in cargo dpkg-deb file install mktemp strip; do
  command -v "$command_name" >/dev/null 2>&1 || {
    echo "Missing packaging tool: $command_name" >&2
    exit 2
  }
done

EXPECTED_FILE_PATTERN=$([ "$DEB_ARCH" = arm64 ] && printf '%s' 'ARM aarch64' || printf '%s' 'x86-64')
BINARY=${IMYEMAIL_CLOUD_LINUX_BINARY:-$ROOT/target/release/imyemail-cloud}
if [ -z "${IMYEMAIL_CLOUD_LINUX_BINARY:-}" ]; then
  cargo build --manifest-path "$ROOT/Cargo.toml" --locked --release --features gtk,secret,webkit
fi
file "$BINARY" | grep -q "$EXPECTED_FILE_PATTERN" || {
  echo "Binary architecture does not match package architecture: $(file "$BINARY")" >&2
  exit 1
}

STAGE=$(mktemp -d "${TMPDIR:-/tmp}/linux-deb.XXXXXX")
cleanup() {
  find "$STAGE" -depth -delete 2>/dev/null || true
}
trap cleanup EXIT HUP INT TERM

install -d \
  "$STAGE/DEBIAN" \
  "$STAGE/usr/bin" \
  "$STAGE/usr/share/applications" \
  "$STAGE/usr/share/metainfo" \
  "$STAGE/usr/share/icons/hicolor/scalable/apps" \
  "$STAGE/usr/share/icons/hicolor/symbolic/apps" \
  "$STAGE/usr/share/doc/imyemail-cloud-native"

install -m 0755 "$BINARY" "$STAGE/usr/bin/imyemail-cloud"
strip "$STAGE/usr/bin/imyemail-cloud"
install -m 0644 "$ROOT/data/email.imy.cloud.desktop" "$STAGE/usr/share/applications/"
install -m 0644 "$ROOT/data/email.imy.cloud.metainfo.xml" "$STAGE/usr/share/metainfo/"
install -m 0644 "$ROOT/data/icons/email.imy.cloud.svg" "$STAGE/usr/share/icons/hicolor/scalable/apps/"
install -m 0644 "$ROOT/data/icons/email.imy.cloud-symbolic.svg" "$STAGE/usr/share/icons/hicolor/symbolic/apps/"

for size in 16 24 32 48 64 128 256 512; do
  destination="$STAGE/usr/share/icons/hicolor/${size}x${size}/apps"
  install -d "$destination"
  install -m 0644 "$ROOT/data/icons/hicolor/${size}x${size}/apps/email.imy.cloud.png" "$destination/"
done

install -m 0644 "$ROOT/packaging/README.en.md" "$STAGE/usr/share/doc/imyemail-cloud-native/README.md"
install -m 0644 "$ROOT/packaging/README.zh-CN.md" "$STAGE/usr/share/doc/imyemail-cloud-native/README.zh-CN.md"
install -m 0644 "$ROOT/LICENSE" "$STAGE/usr/share/doc/imyemail-cloud-native/copyright"
ln -s imyemail-cloud "$STAGE/usr/bin/imyemail-cloud-native"

INSTALLED_SIZE=$(du -sk "$STAGE/usr" | awk '{print $1}')
cat >"$STAGE/DEBIAN/control" <<EOF
Package: imyemail-cloud-native
Version: $VERSION
Section: mail
Priority: optional
Architecture: $DEB_ARCH
Maintainer: imyemail-cloud <hello@imy.email>
Homepage: https://imy.email
Installed-Size: $INSTALLED_SIZE
Depends: libc6 (>= 2.34), libgcc-s1, libgtk-4-1 (>= 4.14), libadwaita-1-0 (>= 1.5), libsecret-1-0, libwebkitgtk-6.0-4
Recommends: gnome-keyring, desktop-file-utils
Breaks: imyemail-cloud (<< 0.2.3)
Replaces: imyemail-cloud (<< 0.2.3)
Description: imyemail-cloud-native client for Linux
 A GTK4 and libadwaita email client using the shared imyemail-cloud mail engine.
 Remote images are blocked by default and may be enabled for HTTPS images.
EOF

cat >"$STAGE/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 0755 "$STAGE/DEBIAN/postinst"

cat >"$STAGE/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 0755 "$STAGE/DEBIAN/postrm"

OUTPUT_DIR=${IMYEMAIL_CLOUD_LINUX_OUTPUT_DIR:-$ROOT/dist}
install -d "$OUTPUT_DIR"
OUTPUT="$OUTPUT_DIR/imyemail-cloud-native-linux-$FILE_ARCH-$BUILD_DATE.deb"
dpkg-deb --build --root-owner-group "$STAGE" "$OUTPUT"
sha256sum "$OUTPUT" >"$OUTPUT.sha256"
printf '%s\n' "$OUTPUT"
