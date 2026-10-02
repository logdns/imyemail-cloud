#!/bin/sh
set -eu
buildroot="$1"
sourceroot="$2"
output="$3"
buildtype="$4"

export CARGO_HOME="${CARGO_HOME:-$buildroot/cargo-home}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$buildroot/cargo-target}"
mkdir -p "$CARGO_HOME" "$CARGO_TARGET_DIR"

features="--features gtk"
profile=debug
if [ "$buildtype" = "release" ]; then
  cargo build --manifest-path "$sourceroot/Cargo.toml" --release $features
  profile=release
else
  cargo build --manifest-path "$sourceroot/Cargo.toml" $features
fi

cp "$CARGO_TARGET_DIR/$profile/imyemail-cloud" "$output"
