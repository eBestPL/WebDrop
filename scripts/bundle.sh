#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=$PWD=WebDrop"
./scripts/cargo.sh build --release --locked
app="$PWD/dist/WebDrop.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" artifacts/AppIcon.iconset
cp target/release/image-optimizer "$app/Contents/MacOS/"
cp macos/Info.plist "$app/Contents/Info.plist"
cp -R macos/en.lproj macos/pl.lproj "$app/Contents/Resources/"
python3 scripts/bundle-codecs.py "$app"
python3 scripts/licenses.py
cp LICENSE README.md README.en.md artifacts/THIRD-PARTY-NOTICES.txt "$app/Contents/Resources/"
mkdir -p "$app/Contents/Resources/docs"
cp -R docs/screenshots "$app/Contents/Resources/docs/"
mkdir -p .tools
CLANG_MODULE_CACHE_PATH="$PWD/.tools/clang-cache" swiftc -framework AppKit scripts/make-icon.swift -o .tools/make-icon
.tools/make-icon "$PWD/artifacts/AppIcon.iconset"
iconutil -c icns artifacts/AppIcon.iconset -o "$app/Contents/Resources/AppIcon.icns"
printf 'APPL????' > "$app/Contents/PkgInfo"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"
printf '\nGotowa aplikacja: %s\n' "$app"
