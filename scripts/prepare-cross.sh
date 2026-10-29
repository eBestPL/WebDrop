#!/bin/sh
# Cross build host: Apple Silicon macOS, Rust 1.95+, Python 3 and Xcode CLI tools.
set -eu
cd "$(dirname "$0")/.."
if [ -x .tools/cargo/bin/rustup ]; then
    export CARGO_HOME="$PWD/.tools/cargo"
    export RUSTUP_HOME="$PWD/.tools/rustup"
    export PATH="$CARGO_HOME/bin:$PATH"
fi
rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-gnu
python3 scripts/fetch-cross-deps.py
