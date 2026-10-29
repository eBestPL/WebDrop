#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec "$root/.tools/cross/zig-aarch64-macos-0.15.2/zig" ar "$@"
