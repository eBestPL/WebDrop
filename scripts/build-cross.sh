#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export WEBDROP_PROJECT_ROOT="$PWD"
mkdir -p .tools/cross/bin
ln -sf "$PWD/scripts/cross-dlltool.sh" .tools/cross/bin/x86_64-w64-mingw32-dlltool
export PATH="$PWD/.tools/cross/bin:$PATH"
export WEBDROP_CROSS_TARGET="${1:?Usage: build-cross.sh linux|windows}"
export CC="$PWD/scripts/cross-cc.sh"
export AR="$PWD/scripts/cross-ar.sh"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$CC"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$CC"
export ZIG_GLOBAL_CACHE_DIR="$PWD/.tools/cross/zig-cache"
export ZIG_LOCAL_CACHE_DIR="$PWD/.tools/cross/zig-local-cache"
export CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=$PWD=WebDrop"
case "$WEBDROP_CROSS_TARGET" in
  linux)
    target=x86_64-unknown-linux-gnu
    export WEBDROP_NATIVE_PREFIX="$PWD/.tools/cross/linux/usr"
    export WEBDROP_NATIVE_LIB_DIR="$WEBDROP_NATIVE_PREFIX/lib/x86_64-linux-gnu"
    ;;
  windows)
    target=x86_64-pc-windows-gnu
    export WEBDROP_NATIVE_PREFIX="$PWD/.tools/cross/windows/mingw64"
    export WEBDROP_NATIVE_LIB_DIR="$WEBDROP_NATIVE_PREFIX/lib"
    ;;
  *) exit 2 ;;
esac
exec ./scripts/cargo.sh build --release --locked --target "$target"
