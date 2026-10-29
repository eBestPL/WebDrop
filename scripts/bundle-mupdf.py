"""Bundle a pinned standalone macOS 15 MuPDF tool, its source references and licenses."""
from pathlib import Path
import hashlib
import json
import shutil
import subprocess
import sys
import tarfile
import urllib.request

root = Path(__file__).resolve().parent.parent
cache = root / '.tools/mupdf'
cache.mkdir(parents=True, exist_ok=True)
app = Path(sys.argv[1]).resolve()
bottle_sha = '7d36f91531459dfc0eb02220cccad03ca6346c58bbd6e46f7d66a45160df0137'
source_sha = '98a5c10cda20c3992cdf76ff6b2a1149c32bd79cc796d3f703230b1185b7e934'
source_url = 'https://mupdf.com/downloads/archive/mupdf-1.28.5-source.tar.gz'
bottle = cache / 'mupdf.tar.gz'
if not bottle.exists():
    token = json.load(urllib.request.urlopen('https://ghcr.io/token?service=ghcr.io&scope=repository:homebrew/core/mupdf-tools:pull'))['token']
    request = urllib.request.Request('https://ghcr.io/v2/homebrew/core/mupdf-tools/blobs/sha256:' + bottle_sha, headers={'Authorization': 'Bearer ' + token})
    bottle.write_bytes(urllib.request.urlopen(request).read())
if hashlib.sha256(bottle.read_bytes()).hexdigest() != bottle_sha:
    raise RuntimeError('MuPDF bottle checksum mismatch')
package = cache / 'mupdf-tools/1.28.5'
if not package.exists():
    with tarfile.open(bottle) as archive:
        archive.extractall(cache)
binary = package / 'bin/mutool'
with tarfile.open(bottle) as archive:
    expected_binary = archive.extractfile('mupdf-tools/1.28.5/bin/mutool').read()
if hashlib.sha256(binary.read_bytes()).digest() != hashlib.sha256(expected_binary).digest():
    raise RuntimeError('Cached MuPDF executable does not match the verified bottle')
dependencies = subprocess.check_output(['otool', '-L', str(binary)], text=True).splitlines()[1:]
if any(not line.strip().startswith(('/usr/lib/', '/System/Library/')) for line in dependencies):
    raise RuntimeError('MuPDF has non-system runtime dependencies')
load = subprocess.check_output(['otool', '-l', str(binary)], text=True)
versions = [tuple(map(int, line.strip().split()[1].split('.'))) for line in load.splitlines() if line.strip().startswith('minos ')]
if not versions or max(versions) > (15, 0):
    raise RuntimeError('MuPDF must support macOS 15')
source = cache / 'mupdf-1.28.5-source.tar.gz'
if not source.exists():
    urllib.request.urlretrieve(source_url, source)
if hashlib.sha256(source.read_bytes()).hexdigest() != source_sha:
    raise RuntimeError('MuPDF source checksum mismatch')
sections = ['MuPDF 1.28.5 — separate vector SVG conversion executable.\nAGPL-3.0-or-later; embedded components retain their respective licenses.\n']
with tarfile.open(source) as archive:
    for member in archive.getmembers():
        if member.isfile() and Path(member.name).name.upper().startswith(('LICENSE', 'COPYING', 'COPYRIGHT', 'NOTICE')):
            sections.append(f'\n--- {member.name} ---\n' + archive.extractfile(member).read().decode(errors='replace'))
source_info = f'''MuPDF 1.28.5 corresponding source and build instructions

Unmodified upstream source, including bundled third-party dependencies:
{source_url}
SHA-256: {source_sha}

Binary: Homebrew arm64_sequoia bottle, macOS 15 Apple Silicon.
https://ghcr.io/v2/homebrew/core/mupdf-tools/blobs/sha256:{bottle_sha}
SHA-256: {bottle_sha}

Build recipe from the bottle (Xcode Command Line Tools and GNU make):
{(package / '.brew/mupdf-tools.rb').read_text()}

To build the tools directly from the source archive on macOS:
MACOSX_DEPLOYMENT_TARGET=15.0 make build=release HAVE_X11=no HAVE_GLUT=no
The resulting mutool is in build/release/. No WebDrop source patches are applied.
'''
resources = app / 'Contents/Resources'
(resources / 'MuPDF-LICENSES.txt').write_text('\n'.join(sections))
(resources / 'MuPDF-SOURCE.txt').write_text(source_info)
(root / 'artifacts/MUPDF-NOTICES.txt').write_text('\n'.join(sections) + '\n' + source_info)
target = app / 'Contents/MacOS/mutool'
shutil.copy2(binary, target)
target.chmod(0o755)
subprocess.run(['codesign', '--force', '--sign', '-', str(target)], check=True)
print('Bundled standalone MuPDF 1.28.5 with source and license information.')
