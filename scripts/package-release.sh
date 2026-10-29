#!/bin/sh
# Run after bundle.sh and both cross builds.
set -eu
cd "$(dirname "$0")/.."
python3 scripts/package-cross.py linux
python3 scripts/package-cross.py windows
cp README.md README.pl.md LICENSE artifacts/THIRD-PARTY-NOTICES.txt dist/WebDrop.app/Contents/Resources/
mkdir -p dist/WebDrop.app/Contents/Resources/docs
cp -R docs/screenshots dist/WebDrop.app/Contents/Resources/docs/
codesign --force --sign - dist/WebDrop.app
codesign --verify --deep --strict dist/WebDrop.app
ditto -c -k --sequesterRsrc --keepParent dist/WebDrop.app dist/WebDrop-macOS-arm64.zip
python3 - <<'PY'
from pathlib import Path
import hashlib
folder = Path('dist')
names = ['WebDrop-macOS-arm64.zip', 'WebDrop-Windows-x64.zip', 'WebDrop-Linux-x64.tar.gz']
lines = []
for name in names:
    path = folder / name
    if path.stat().st_size >= 100 * 1024 * 1024:
        raise RuntimeError(f'{name} exceeds the GitHub per-file limit')
    lines.append(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {name}\n')
(folder / 'SHA256SUMS.txt').write_text(''.join(lines))
PY
