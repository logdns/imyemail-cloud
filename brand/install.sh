#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
EXPORT="$ROOT/export"
CODEDEV="$(cd "$ROOT/.." && pwd)"

copy_dir() {
  local src="$1" dest="$2"
  mkdir -p "$dest"
  rsync -a --delete "$src"/ "$dest"/
}

copy_file() {
  mkdir -p "$(dirname "$2")"
  cp "$1" "$2"
}

# Apple
copy_file "$EXPORT/apple/AppIcon.icns" "$CODEDEV/apple/packaging/AppIcon.icns"
copy_dir "$EXPORT/apple/AppIcon.appiconset" "$CODEDEV/apple/packaging/ios/AppIcon.appiconset"
copy_file "$EXPORT/apple/AppIcon-1024.png" "$CODEDEV/apple/packaging/AppIcon-1024.png"
for f in icon_16x16.png icon_16x16@2x.png icon_32x32.png icon_32x32@2x.png icon_128x128.png icon_128x128@2x.png icon_256x256.png icon_256x256@2x.png icon_512x512.png icon_512x512@2x.png; do
  copy_file "$EXPORT/apple/AppIcon.iconset/$f" "$CODEDEV/apple/packaging/macos/AppIcon.appiconset/$f"
done
mkdir -p "$CODEDEV/apple/Sources/ChckDesign/Resources/Media.xcassets/BrandMark.imageset"
copy_file "$EXPORT/master/icon-rounded-1024.png" "$CODEDEV/apple/Sources/ChckDesign/Resources/Media.xcassets/BrandMark.imageset/BrandMark.png"

# Android launcher
for density in mdpi hdpi xhdpi xxhdpi xxxhdpi; do
  copy_file "$EXPORT/android/mipmap-${density}/ic_launcher.png" "$CODEDEV/android/app/src/main/res/mipmap-${density}/ic_launcher.png"
  copy_file "$EXPORT/android/mipmap-${density}/ic_launcher_round.png" "$CODEDEV/android/app/src/main/res/mipmap-${density}/ic_launcher_round.png"
done

# Windows
for f in StoreLogo.png Square44x44Logo.png Square150x150Logo.png Wide310x150Logo.png SplashScreen.png Square71x71Logo.png Square310x310Logo.png \
         Square44x44Logo.targetsize-16.png Square44x44Logo.targetsize-24.png Square44x44Logo.targetsize-32.png \
         Square44x44Logo.targetsize-48.png Square44x44Logo.targetsize-256.png; do
  copy_file "$EXPORT/windows/$f" "$CODEDEV/windows/Chck.Mail.Package/Images/$f"
done
copy_file "$EXPORT/windows/imyemail-cloud.ico" "$CODEDEV/windows/Chck.Mail/Assets/imyemail-cloud.ico"
copy_file "$EXPORT/windows/Square44x44Logo.png" "$CODEDEV/windows/Chck.Mail/Assets/Square44x44Logo.png"
copy_file "$EXPORT/windows/Square150x150Logo.png" "$CODEDEV/windows/Chck.Mail/Assets/Square150x150Logo.png"

# Linux
copy_file "$EXPORT/linux/hicolor/scalable/apps/email.imy.cloud.svg" "$CODEDEV/linux/data/icons/email.imy.cloud.svg"
copy_file "$EXPORT/linux/hicolor/symbolic/apps/email.imy.cloud-symbolic.svg" "$CODEDEV/linux/data/icons/email.imy.cloud-symbolic.svg"
for size in 16 24 32 48 64 128 256 512; do
  copy_file "$EXPORT/linux/hicolor/${size}x${size}/apps/email.imy.cloud.png" \
    "$CODEDEV/linux/data/icons/hicolor/${size}x${size}/apps/email.imy.cloud.png"
done

echo "installed brand assets into clients"
