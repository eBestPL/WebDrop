#!/bin/sh
# macOS only by default. Pass --all explicitly after both cross builds.
set -eu
cd "$(dirname "$0")/.."
if [ ! -d dist/WebDrop.app ]; then
    printf 'Missing dist/WebDrop.app. Run ./scripts/bundle.sh first.\n' >&2
    exit 1
fi
case "${1:-}" in
    "") ;;
    --all)
        python3 scripts/package-cross.py linux
        python3 scripts/package-cross.py windows
        ;;
    *) printf 'Usage: %s [--all]\n' "$0" >&2; exit 2 ;;
esac
cp README.md README.pl.md LICENSE artifacts/THIRD-PARTY-NOTICES.txt dist/WebDrop.app/Contents/Resources/
rm -f dist/WebDrop.app/Contents/Resources/README.en.md
mkdir -p dist/WebDrop.app/Contents/Resources/docs
cp -R docs/screenshots dist/WebDrop.app/Contents/Resources/docs/
cp docs/PACKAGING.md dist/WebDrop.app/Contents/Resources/docs/
codesign --force --sign - dist/WebDrop.app
codesign --verify --deep --strict dist/WebDrop.app
python3 scripts/package-installers.py macos
python3 - <<'PY'
from pathlib import Path
import hashlib
folder = Path('dist')
names = ['WebDrop-macOS-arm64.pkg', 'WebDrop-Windows-x64-Setup.exe', 'WebDrop-x86_64.AppImage']
lines = []
for name in names:
    path = folder / name
    if not path.exists():
        continue
    if path.stat().st_size >= 100 * 1024 * 1024:
        raise RuntimeError(f'{name} exceeds the GitHub per-file limit')
    lines.append(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {name}\n')
(folder / 'SHA256SUMS.txt').write_text(''.join(lines))
PY
