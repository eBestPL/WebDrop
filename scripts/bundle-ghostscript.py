"""Bundle the standalone, source-built PostScript interpreter and its notices."""
from pathlib import Path
import hashlib
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
app = Path(sys.argv[1]).resolve()
cache = root / '.tools/ghostscript'
binary = cache / 'gs'
dependencies = subprocess.check_output(['otool', '-L', str(binary)], text=True).splitlines()[1:]
if any(not line.strip().startswith(('/usr/lib/', '/System/Library/')) for line in dependencies):
    raise RuntimeError('Standalone Ghostscript has non-system runtime dependencies')
load = subprocess.check_output(['otool', '-l', str(binary)], text=True)
versions = [tuple(map(int, line.strip().split()[1].split('.'))) for line in load.splitlines() if line.strip().startswith('minos ')]
if max(versions) > (15, 0):
    raise RuntimeError('Ghostscript must support macOS 15')
target = app / 'Contents/MacOS/gs'
shutil.copy2(binary, target)
target.chmod(0o755)
subprocess.run(['codesign', '--force', '--sign', '-', str(target)], check=True)
source = cache / 'ghostpdl-10.08.0'
sections = ['Ghostscript 10.08.0 — separate EPS rendering executable.\nAGPL-3.0-or-later; embedded components retain their respective licenses.\n']
for path in sorted(source.rglob('*')):
    if path.is_file() and path.name.upper().startswith(('COPYING', 'LICENSE', 'COPYRIGHT', 'NOTICE')):
        sections.append(f'\n--- {path.relative_to(source)} ---\n{path.read_text(errors="replace")}\n')
resources = app / 'Contents/Resources'
(resources / 'Ghostscript-LICENSES.txt').write_text('\n'.join(sections))
archive = cache / 'ghostpdl-10.08.0.tar.xz'
source_info = f'''Ghostscript 10.08.0 corresponding source and build instructions

Unmodified upstream source, including embedded dependencies and fonts:
https://github.com/ArtifexSoftware/ghostpdl-downloads/releases/download/gs10080/ghostpdl-10.08.0.tar.xz
SHA-256: {hashlib.sha256(archive.read_bytes()).hexdigest()}

Build on Apple Silicon with Xcode Command Line Tools. Run this script from the
WebDrop source tree. No source patches are applied. Initialization resources,
ICC profiles and standard fonts are embedded in the executable.

{(root / 'scripts/build-ghostscript.sh').read_text()}
'''
(resources / 'Ghostscript-SOURCE.txt').write_text(source_info)
(root / 'artifacts/GHOSTSCRIPT-NOTICES.txt').write_text('\n'.join(sections) + '\n' + source_info)
print('Bundled standalone Ghostscript 10.08.0 with embedded fonts and license/source information.')
