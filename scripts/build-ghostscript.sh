#!/bin/sh
# Standalone EPS interpreter with embedded fonts/resources and no Homebrew runtime.
set -eu
cd "$(dirname "$0")/.."
python3 - <<'PY'
from pathlib import Path
import hashlib, tarfile, urllib.request
root = Path('.tools/ghostscript')
root.mkdir(parents=True, exist_ok=True)
archive = root / 'ghostpdl-10.08.0.tar.xz'
url = 'https://github.com/ArtifexSoftware/ghostpdl-downloads/releases/download/gs10080/ghostpdl-10.08.0.tar.xz'
if not archive.exists():
    urllib.request.urlretrieve(url, archive)
if hashlib.sha256(archive.read_bytes()).hexdigest() != '94943e3c4adb7115d132b3c28b913672f2e62e6a9d802f7e754e716b1634174a':
    raise RuntimeError('Ghostscript source checksum mismatch')
if not (root / 'ghostpdl-10.08.0').exists():
    with tarfile.open(archive) as source:
        source.extractall(root)
PY
gs_cache="$PWD/.tools/ghostscript"
if [ -x "$gs_cache/gs" ] && [ -f "$gs_cache/pdfwrite-enabled" ]; then
    exit 0
fi
gs_build=$(mktemp -d /tmp/webdrop-ghostscript.XXXXXX)
trap 'rm -rf "$gs_build"' EXIT
cp -R "$gs_cache/ghostpdl-10.08.0/." "$gs_build/"
cd "$gs_build"
MACOSX_DEPLOYMENT_TARGET=15.0 CFLAGS="-O2 -mmacosx-version-min=15.0" LDFLAGS="-mmacosx-version-min=15.0" \
./configure --disable-cups --disable-gtk --disable-dbus --disable-fontconfig \
    --disable-contrib --without-tesseract --without-libidn --without-libpaper \
    --without-ijs --without-x --without-pdf --with-drivers=pngalpha,pdfwrite
make -j"$(sysctl -n hw.logicalcpu)" gs
cp bin/gs "$gs_cache/gs"

touch "$gs_cache/pdfwrite-enabled"
