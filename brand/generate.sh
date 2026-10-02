#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
SVG="$ROOT/svg"
OUT="$ROOT/export"
RSVG="${RSVG:-$(command -v rsvg-convert)}"

if [[ -z "$RSVG" ]]; then
  echo "rsvg-convert not found" >&2
  exit 1
fi

png() {
  local src="$1" dest="$2" w="$3" h="${4:-$3}"
  mkdir -p "$(dirname "$dest")"
  "$RSVG" -w "$w" -h "$h" -o "$dest" "$src"
}

copy() {
  mkdir -p "$(dirname "$2")"
  cp "$1" "$2"
}

write_ico() {
  python3 - "$@" <<'PY'
import struct, sys, pathlib, zlib

def parse_png(path):
    data = pathlib.Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    w = struct.unpack(">I", data[16:20])[0]
    h = struct.unpack(">I", data[20:24])[0]
    return w, h, data

out = pathlib.Path(sys.argv[1])
images = [parse_png(p) for p in sys.argv[2:]]
header = struct.pack("<HHH", 0, 1, len(images))
offset = 6 + 16 * len(images)
entries = b""
payload = b""
for w, h, data in images:
    entries += struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(data), offset)
    payload += data
    offset += len(data)
out.write_bytes(header + entries + payload)
PY
}

rm -rf "$OUT"
mkdir -p "$OUT"

# Canonical rasters
png "$SVG/icon-fullbleed.svg" "$OUT/master/icon-1024.png" 1024
png "$SVG/icon-rounded.svg" "$OUT/master/icon-rounded-1024.png" 1024
png "$SVG/icon-foreground.svg" "$OUT/master/icon-foreground-1024.png" 1024
png "$SVG/icon-mono.svg" "$OUT/master/icon-mono-1024.png" 1024
png "$SVG/icon-notification.svg" "$OUT/master/icon-notification-1024.png" 1024
png "$SVG/empty-inbox.svg" "$OUT/master/empty-inbox-512.png" 512
copy "$SVG/icon-fullbleed.svg" "$OUT/master/icon-fullbleed.svg"
copy "$SVG/icon-rounded.svg" "$OUT/master/icon-rounded.svg"
copy "$SVG/icon-foreground.svg" "$OUT/master/icon-foreground.svg"
copy "$SVG/icon-mono.svg" "$OUT/master/icon-mono.svg"
copy "$SVG/icon-notification.svg" "$OUT/master/icon-notification.svg"
copy "$SVG/icon-symbolic.svg" "$OUT/master/icon-symbolic.svg"
copy "$SVG/wide-tile.svg" "$OUT/master/wide-tile.svg"
copy "$SVG/splash.svg" "$OUT/master/splash.svg"
copy "$SVG/empty-inbox.svg" "$OUT/master/empty-inbox.svg"

# Apple macOS AppIcon
MAC_ICONSET="$OUT/apple/AppIcon.iconset"
for size in 16 32 128 256 512; do
  png "$SVG/icon-rounded.svg" "$MAC_ICONSET/icon_${size}x${size}.png" "$size"
  png "$SVG/icon-rounded.svg" "$MAC_ICONSET/icon_${size}x${size}@2x.png" "$((size * 2))"
done
iconutil -c icns -o "$OUT/apple/AppIcon.icns" "$MAC_ICONSET"
png "$SVG/icon-rounded.svg" "$OUT/apple/AppIcon-1024.png" 1024

# Apple iOS AppIcon (no rounding; iOS applies mask)
IOS="$OUT/apple/AppIcon.appiconset"
python3 - "$SVG/icon-fullbleed.svg" "$IOS" <<'PY'
import json, os, subprocess, sys
from pathlib import Path
src, dest = sys.argv[1], Path(sys.argv[2])
dest.mkdir(parents=True, exist_ok=True)
rsvg = os.environ.get("RSVG", "rsvg-convert")
images = []
specs = [
    (20, 2, "iphone"),
    (20, 3, "iphone"),
    (29, 2, "iphone"),
    (29, 3, "iphone"),
    (40, 2, "iphone"),
    (40, 3, "iphone"),
    (60, 2, "iphone"),
    (60, 3, "iphone"),
    (20, 1, "ipad"),
    (20, 2, "ipad"),
    (29, 1, "ipad"),
    (29, 2, "ipad"),
    (40, 1, "ipad"),
    (40, 2, "ipad"),
    (76, 1, "ipad"),
    (76, 2, "ipad"),
    (83.5, 2, "ipad"),
    (1024, 1, "ios-marketing"),
]
for size, scale, idiom in specs:
    px = int(round(size * scale))
    name = f"icon-{px}.png" if size != 83.5 else "icon-167.png"
    out = dest / name
    subprocess.check_call([rsvg, "-w", str(px), "-h", str(px), "-o", str(out), src])
    images.append({
        "filename": name,
        "idiom": idiom,
        "scale": f"{scale}x",
        "size": f"{size:g}x{size:g}",
    })
# de-dup filenames that share pixels
seen = set()
uniq = []
for img in images:
    key = (img["filename"], img["idiom"], img["scale"], img["size"])
    if key in seen:
        continue
    seen.add(key)
    uniq.append(img)
(dest / "Contents.json").write_text(json.dumps({
    "images": uniq,
    "info": {"author": "imy.email", "version": 1},
}, indent=2) + "\n")
PY

# Android mipmaps + notification
AND="$OUT/android"
for density in "mdpi:48" "hdpi:72" "xhdpi:96" "xxhdpi:144" "xxxhdpi:192"; do
  name="${density%%:*}"
  size="${density##*:}"
  png "$SVG/icon-rounded.svg" "$AND/mipmap-${name}/ic_launcher.png" "$size"
  png "$SVG/icon-rounded.svg" "$AND/mipmap-${name}/ic_launcher_round.png" "$size"
done
png "$SVG/icon-foreground.svg" "$AND/drawable/ic_launcher_foreground.png" 432
png "$SVG/icon-fullbleed.svg" "$AND/drawable/ic_launcher_background.png" 432
png "$SVG/icon-notification.svg" "$AND/drawable-mdpi/ic_stat_imyemail_cloud.png" 24
png "$SVG/icon-notification.svg" "$AND/drawable-hdpi/ic_stat_imyemail_cloud.png" 36
png "$SVG/icon-notification.svg" "$AND/drawable-xhdpi/ic_stat_imyemail_cloud.png" 48
png "$SVG/icon-notification.svg" "$AND/drawable-xxhdpi/ic_stat_imyemail_cloud.png" 72
png "$SVG/icon-notification.svg" "$AND/drawable-xxxhdpi/ic_stat_imyemail_cloud.png" 96
png "$SVG/empty-inbox.svg" "$AND/drawable-xxhdpi/ic_empty_inbox.png" 256

# Windows
WIN="$OUT/windows"
png "$SVG/icon-fullbleed.svg" "$WIN/StoreLogo.png" 50
png "$SVG/icon-fullbleed.svg" "$WIN/Square44x44Logo.png" 44
png "$SVG/icon-fullbleed.svg" "$WIN/Square44x44Logo.targetsize-16.png" 16
png "$SVG/icon-fullbleed.svg" "$WIN/Square44x44Logo.targetsize-24.png" 24
png "$SVG/icon-fullbleed.svg" "$WIN/Square44x44Logo.targetsize-32.png" 32
png "$SVG/icon-fullbleed.svg" "$WIN/Square44x44Logo.targetsize-48.png" 48
png "$SVG/icon-fullbleed.svg" "$WIN/Square44x44Logo.targetsize-256.png" 256
png "$SVG/icon-fullbleed.svg" "$WIN/Square150x150Logo.png" 150
png "$SVG/wide-tile.svg" "$WIN/Wide310x150Logo.png" 310 150
png "$SVG/splash.svg" "$WIN/SplashScreen.png" 620 300
png "$SVG/icon-fullbleed.svg" "$WIN/Square71x71Logo.png" 71
png "$SVG/icon-fullbleed.svg" "$WIN/Square310x310Logo.png" 310
png "$SVG/icon-rounded.svg" "$WIN/app-256.png" 256
png "$SVG/icon-rounded.svg" "$WIN/app-128.png" 128
png "$SVG/icon-rounded.svg" "$WIN/app-64.png" 64
png "$SVG/icon-rounded.svg" "$WIN/app-48.png" 48
png "$SVG/icon-rounded.svg" "$WIN/app-32.png" 32
png "$SVG/icon-rounded.svg" "$WIN/app-16.png" 16
write_ico "$WIN/imyemail-cloud.ico" \
  "$WIN/app-16.png" "$WIN/app-32.png" "$WIN/app-48.png" "$WIN/app-64.png" "$WIN/app-128.png" "$WIN/app-256.png"

# Linux hicolor
LIN="$OUT/linux"
copy "$SVG/icon-rounded.svg" "$LIN/hicolor/scalable/apps/email.imy.cloud.svg"
copy "$SVG/icon-symbolic.svg" "$LIN/hicolor/symbolic/apps/email.imy.cloud-symbolic.svg"
for size in 16 24 32 48 64 128 256 512; do
  png "$SVG/icon-rounded.svg" "$LIN/hicolor/${size}x${size}/apps/email.imy.cloud.png" "$size"
done

echo "generated $OUT"
find "$OUT" -type f | wc -l
