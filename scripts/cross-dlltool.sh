#!/bin/sh
set -eu
root="${WEBDROP_PROJECT_ROOT:?}"
exec "$root/.tools/cross/zig-aarch64-macos-0.15.2/zig" dlltool "$@"
