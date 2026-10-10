#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
platform=${1:?usage: package.sh GOOS/GOARCH}
case "$platform" in
  darwin/arm64|darwin/amd64|linux/arm64|linux/amd64|windows/arm64|windows/amd64) ;;
  *) printf '%s\n' 'unsupported platform' >&2; exit 1 ;;
esac
goos=${platform%/*}
goarch=${platform#*/}
test "$(go env GOOS)/$(go env GOARCH)" = "$platform"
mkdir -p resources/licenses dist/release
suffix=
if [[ "$goos" == windows ]]; then suffix=.exe; fi
cargo build --manifest-path ../core/Cargo.toml --locked --release -p chck-cli --features desktop-vault
core_target=${CARGO_TARGET_DIR:-../core/target}
cp "$core_target/release/imyemail-cloud$suffix" "resources/imyemail-cloud-core$suffix"
cp ../LICENSE resources/LICENSE
cp THIRD-PARTY-NOTICES.txt resources/THIRD-PARTY-NOTICES.txt
for module in github.com/egoist/mygo github.com/ebitengine/purego github.com/go-text/typesetting golang.org/x/image; do
  directory=$(go list -m -f '{{.Dir}}' "$module")
  name=${module//\//_}
  cp "$directory/LICENSE" "resources/licenses/$name.txt"
done
target=$(rustc -vV | sed -n 's/^host: //p')
cargo metadata --manifest-path ../core/Cargo.toml --locked --format-version 1 \
  --filter-platform "$target" --features chck-cli/desktop-vault > dist/cargo-metadata.json
python3 scripts/licenses.py dist/cargo-metadata.json resources/licenses
go tool mygo build -platform "$platform" -skip-dmg -skip-notarize

output="dist/$goos-$goarch"
version=$(python3 -c 'import json; print(json.load(open("mygo.json"))["version"])')
asset="imyemail-cloud-mygo-$version-$goos-$goarch"
if [[ "$goos" == darwin ]]; then
  /usr/bin/codesign --verify --deep --strict "$output/imyemail-cloud-mygo.app"
  /usr/bin/ditto -c -k --sequesterRsrc --keepParent "$output/imyemail-cloud-mygo.app" "dist/release/$asset-adhoc.zip"
elif [[ "$goos" == windows ]]; then
  python3 - "$output" "dist/release/$asset-unsigned.zip" <<'PY'
import pathlib
import sys
import zipfile
root = pathlib.Path(sys.argv[1])
with zipfile.ZipFile(sys.argv[2], "w", zipfile.ZIP_DEFLATED) as archive:
    for path in sorted(root.rglob("*")):
        if path.is_file() and "Setup" not in path.name:
            archive.write(path, path.relative_to(root))
PY
else
  cp "$output"/*.deb "dist/release/$asset.deb"
  cp "$output"/*.tar.gz "dist/release/$asset.tar.gz"
fi
python3 - dist/release <<'PY'
import hashlib
import pathlib
import sys
for path in sorted(pathlib.Path(sys.argv[1]).iterdir()):
    if path.suffix != ".sha256":
        digest = hashlib.file_digest(path.open("rb"), "sha256").hexdigest()
        path.with_name(path.name + ".sha256").write_text(digest + "  " + path.name + "\n")
PY
