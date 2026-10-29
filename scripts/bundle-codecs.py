"""Bundle the complete native codec dylib closure; no Homebrew needed at runtime."""
import os
import plistlib
from pathlib import Path
import shutil
import subprocess
import sys

app = Path(sys.argv[1]).resolve()
executable = app / 'Contents/MacOS/image-optimizer'
frameworks = app / 'Contents/Frameworks'
frameworks.mkdir(parents=True, exist_ok=True)
root = Path(__file__).resolve().parent.parent
prefix = Path(os.environ.get('WEBDROP_BREW_PREFIX', '/opt/homebrew' if Path('/opt/homebrew').exists() else '/usr/local'))
deployment_target = (15, 0)


def minimum_os(path):
    load = subprocess.check_output(['otool', '-l', str(path)], text=True).splitlines()
    versions = [line.strip().split()[1] for line in load if line.strip().startswith('minos ')]
    return max((tuple(map(int, version.split('.'))) for version in versions), default=(13, 0))


def dependencies(path):
    lines = subprocess.check_output(['otool', '-L', str(path)], text=True).splitlines()[1:]
    # First entry of a dylib is its install name, not a dependency.
    if path.suffix == '.dylib':
        lines = lines[1:]
    return [line.strip().split(' (compatibility')[0] for line in lines]


def resolve(name, source):
    if name.startswith('/usr/lib/') or name.startswith('/System/Library/'):
        return None
    if name.startswith('@rpath/'):
        candidate = source.parent / name[len('@rpath/'):]
    elif name.startswith('@loader_path/'):
        candidate = source.parent / name[len('@loader_path/'):]
    elif name.startswith('/'):
        candidate = Path(name)
    else:
        raise RuntimeError(f'Unresolved dependency: {name}')
    if not candidate.is_file():
        raise RuntimeError(f'Missing dependency: {candidate}')
    # Homebrew may retain older compatible kegs after upgrading a dependency.
    # Keep the documented macOS 15 baseline, using the same dylib ABI/install name.
    if minimum_os(candidate) > deployment_target:
        actual = candidate.resolve()
        formula = next((p for p in actual.parents if p.parent == prefix / 'Cellar'), None)
        compatible = [] if formula is None else [
            p for p in formula.glob('*/lib/' + candidate.name)
            if p.is_file() and minimum_os(p) <= deployment_target
        ]
        if not compatible:
            raise RuntimeError(f'{candidate} requires newer macOS; install a macOS 15-compatible bottle before packaging')
        candidate = sorted(compatible)[-1]
    return candidate


copied = {}
formula_roots = set()


def visit(source, destination, is_main=False):
    for name in dependencies(source):
        original = resolve(name, source)
        if original is None:
            continue
        filename = original.name
        actual = original.resolve()
        if filename in copied and copied[filename] != actual:
            raise RuntimeError(f'Conflicting libraries: {filename}')
        if filename not in copied:
            copied[filename] = actual
            target = frameworks / filename
            shutil.copy2(actual, target)
            target.chmod(0o755)
            for parent in actual.parents:
                if parent.parent.parent == prefix / 'Cellar':
                    formula_roots.add(parent)
                    break
            visit(original, target)
            subprocess.run(['install_name_tool', '-id', f'@rpath/{filename}', str(target)], check=True)
        replacement = f'@executable_path/../Frameworks/{filename}' if is_main else f'@loader_path/{filename}'
        subprocess.run(['install_name_tool', '-change', name, replacement, str(destination)], check=True)
    # Build-only rpaths must not leak into the distributable bundle.
    load = subprocess.check_output(['otool', '-l', str(destination)], text=True).splitlines()
    for i, line in enumerate(load):
        if line.strip() == 'cmd LC_RPATH':
            rpath = load[i + 2].strip().removeprefix('path ').split(' (offset')[0]
            if rpath.startswith(str(prefix)):
                subprocess.run(['install_name_tool', '-delete_rpath', rpath, str(destination)], check=True)


visit(executable, executable, True)
sections = ['Native codecs bundled with WebDrop. Built from Homebrew packages; all runtime dependencies are included.\n']
for formula in sorted(formula_roots):
    sections.append(f'\n{"=" * 72}\n{formula.parent.name} {formula.name}\n')
    licenses = [p for p in formula.iterdir() if p.is_file() and p.name.upper().startswith(('LICENSE', 'COPYING', 'COPYRIGHT', 'NOTICE', 'PATENT', 'AUTHORS'))]
    if not licenses:
        raise RuntimeError(f'No license found for {formula}')
    for path in sorted(licenses):
        sections.append(f'\n--- {path.name} ---\n{path.read_text(errors="replace")}\n')
(root / 'artifacts/NATIVE-CODEC-NOTICES.txt').write_text('\n'.join(sections))
for name in sorted(copied):
    path = frameworks / name
    for dep in dependencies(path):
        if not dep.startswith(('@loader_path/', '/usr/lib/', '/System/Library/')):
            raise RuntimeError(f'External runtime dependency: {path}: {dep}')
    subprocess.run(['codesign', '--force', '--sign', '-', str(path)], check=True)
# Match the bundle's declared minimum OS to the linked native libraries.
minimum = max(minimum_os(p) for p in [executable, *(frameworks / name for name in copied)])
plist_path = app / 'Contents/Info.plist'
with plist_path.open('rb') as file:
    plist = plistlib.load(file)
plist['LSMinimumSystemVersion'] = '.'.join(map(str, minimum))
with plist_path.open('wb') as file:
    plistlib.dump(plist, file, sort_keys=False)
print(f'Bundled {len(copied)} native libraries with license notices; macOS {plist["LSMinimumSystemVersion"]}+.')
