"""Create one downloadable file per OS; no manual archive extraction required."""
from pathlib import Path
import hashlib
import json
import plistlib
import re
import shutil
import struct
import subprocess
import sys
import tarfile
import urllib.request
import xml.sax.saxutils as xml

ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / 'dist'
WORK = ROOT / 'artifacts' / 'installers'
VERSION = re.search(r'^version = "([^"]+)"', (ROOT / 'Cargo.toml').read_text(), re.M)[1]
RUNTIME_COMMIT = 'fedfad51acc2860b2916a1eef32ba4a500ff31ef'
RUNTIME_SHA = '617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46'
SOURCE_SHA = 'e27ea3c837e9c660e1a9a2993b673ae93bbb53077330efba8654031e302d4e8e'


def run(*args):
    subprocess.run([str(a) for a in args], check=True)


def fetch(name, url, digest):
    path = ROOT / '.tools' / 'appimage' / name
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        with urllib.request.urlopen(url, timeout=60) as response:
            data = response.read()
        if hashlib.sha256(data).hexdigest() != digest:
            raise RuntimeError(f'{name}: download checksum mismatch; update the pinned runtime explicitly')
        path.write_bytes(data)
    if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise RuntimeError(f'{name}: cached checksum mismatch')
    return path


def icon():
    path = ROOT / 'artifacts' / 'AppIcon.iconset' / 'icon_256x256.png'
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        run('sips', '-s', 'format', 'png', '-z', '256', '256',
            DIST / 'WebDrop.app/Contents/Resources/AppIcon.icns', '--out', path)
    return path


def macos():
    app = DIST / 'WebDrop.app'
    run('codesign', '--verify', '--deep', '--strict', app)
    work = WORK / 'macos'
    payload = work / 'root'
    if payload.exists():
        shutil.rmtree(payload)
    (payload / 'Applications').mkdir(parents=True)
    run('ditto', app, payload / 'Applications/WebDrop.app')
    components = work / 'components.plist'
    components.write_bytes(plistlib.dumps([{
        'RootRelativeBundlePath': 'Applications/WebDrop.app',
        'BundleIsRelocatable': False,
        'BundleIsVersionChecked': True,
        'BundleHasStrictIdentifier': True,
        'BundleOverwriteAction': 'upgrade',
    }]))
    package = work / 'WebDrop-component.pkg'
    run('pkgbuild', '--root', payload, '--component-plist', components,
        '--identifier', 'pl.ebest.webdrop.pkg', '--version', VERSION,
        '--install-location', '/', '--ownership', 'recommended', package)
    distribution = work / 'distribution.xml'
    distribution.write_text(f'''<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
  <title>WebDrop Image Optimizer</title>
  <options customize="never" require-scripts="false" hostArchitectures="arm64"/>
  <domains enable_anywhere="false" enable_currentUserHome="false" enable_localSystem="true"/>
  <volume-check><allowed-os-versions><os-version min="15.0"/></allowed-os-versions></volume-check>
  <choices-outline><line choice="webdrop"/></choices-outline>
  <choice id="webdrop" visible="false" title="WebDrop"><pkg-ref id="pl.ebest.webdrop.pkg"/></choice>
  <pkg-ref id="pl.ebest.webdrop.pkg" version="{xml.escape(VERSION)}" onConclusion="none">WebDrop-component.pkg</pkg-ref>
</installer-gui-script>
''')
    output = DIST / 'WebDrop-macOS-arm64.pkg'
    run('productbuild', '--distribution', distribution, '--package-path', work, output)
    return output


def windows():
    payload = ROOT / 'artifacts/packages/WebDrop-Windows-x64'
    if not (payload / 'WebDrop.exe').exists():
        raise RuntimeError('Stage the Windows payload with package-cross.py first')
    (payload / 'native-licenses/NSIS.txt').write_bytes(subprocess.check_output(['makensis', '-LICENSE']))
    # Windows Vista+ ICO files can contain PNG images. Used by the Start menu shortcut.
    png = icon().read_bytes()
    (payload / 'WebDrop.ico').write_bytes(struct.pack('<HHH', 0, 1, 1)
        + struct.pack('<BBBBHHII', 0, 0, 0, 0, 1, 32, len(png), 22) + png)
    work = WORK / 'windows'
    work.mkdir(parents=True, exist_ok=True)
    uninstall = work / 'remove-files.nsh'
    def quote(value):
        return str(value).replace('$', '$$').replace('"', '$\\"')
    files = sorted(p for p in payload.rglob('*') if p.is_file())
    dirs = sorted((p for p in payload.rglob('*') if p.is_dir()), key=lambda p: len(p.parts), reverse=True)
    uninstall.write_text('\n'.join(
        ['Delete "$INSTDIR\\' + quote(str(p.relative_to(payload)).replace('/', '\\')) + '"' for p in files]
        + ['RMDir "$INSTDIR\\' + quote(str(p.relative_to(payload)).replace('/', '\\')) + '"' for p in dirs]) + '\n')
    output = DIST / 'WebDrop-Windows-x64-Setup.exe'
    run('makensis', '-V2', f'-DVERSION={VERSION}', f'-DPAYLOAD={payload}',
        f'-DOUTPUT={output}', f'-DREMOVE_FILES={uninstall}', ROOT / 'packaging/windows.nsi')
    return output


def linux():
    payload = ROOT / 'artifacts/packages/WebDrop-Linux-x64'
    if not (payload / 'WebDrop').exists():
        raise RuntimeError('Stage the Linux payload with package-cross.py first')
    runtime_url = 'https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64'
    source_url = f'https://codeload.github.com/AppImage/type2-runtime/tar.gz/{RUNTIME_COMMIT}'
    runtime = fetch('runtime-x86_64', runtime_url, RUNTIME_SHA)
    source = fetch('source.tar.gz', source_url, SOURCE_SHA)
    work = WORK / 'linux'
    appdir = work / 'WebDrop.AppDir'
    if appdir.exists():
        shutil.rmtree(appdir)
    shutil.copytree(payload, appdir)
    (appdir / 'WebDrop').chmod(0o755)
    (appdir / 'AppRun').symlink_to('WebDrop')
    shutil.copy2(icon(), appdir / 'webdrop.png')
    (appdir / '.DirIcon').symlink_to('webdrop.png')
    (appdir / 'webdrop.desktop').write_text('''[Desktop Entry]
Type=Application
Name=WebDrop
Comment=Optimize images for the web
Exec=WebDrop %F
Icon=webdrop
Terminal=false
Categories=Graphics;
''')
    with tarfile.open(source) as archive:
        license_data = archive.extractfile(f'type2-runtime-{RUNTIME_COMMIT}/LICENSE').read()
    (appdir / 'native-licenses/AppImage-runtime.txt').write_bytes(license_data)
    sources = appdir / 'native-sources'
    sources.mkdir(exist_ok=True)
    shutil.copy2(source, sources / 'AppImage-runtime-source.tar.gz')
    # Include the LGPL runtime dependency source and the upstream patches/build recipe.
    for filename, url, digest, license_paths in [
        ('fuse-3.15.0.tar.xz', 'https://github.com/libfuse/libfuse/releases/download/fuse-3.15.0/fuse-3.15.0.tar.xz',
         '70589cfd5e1cff7ccd6ac91c86c01be340b227285c5e200baa284e401eea2ca0', ['fuse-3.15.0/LICENSE', 'fuse-3.15.0/LGPL2.txt']),
        ('squashfuse-0.5.2.tar.gz', 'https://github.com/vasi/squashfuse/archive/0.5.2.tar.gz',
         'db0238c5981dabbd80ee09ae15387f390091668ca060a7bc38047912491443d3', ['squashfuse-0.5.2/LICENSE']),
    ]:
        dependency = fetch(filename, url, digest)
        shutil.copy2(dependency, sources / filename)
        with tarfile.open(dependency) as archive:
            for license_path in license_paths:
                (appdir / 'native-licenses' / (license_path.replace('/', '-') + '.txt')).write_bytes(archive.extractfile(license_path).read())
    for filename, url, digest in [
        ('musl-COPYRIGHT.txt', 'https://git.musl-libc.org/cgit/musl/plain/COPYRIGHT?h=v1.2.5',
         'f9bc4423732350eb0b3f7ed7e91d530298476f8fec0c6c427a1c04ade22655af'),
        ('zstd-LICENSE.txt', 'https://raw.githubusercontent.com/facebook/zstd/v1.5.6/LICENSE',
         '7055266497633c9025b777c78eb7235af13922117480ed5c674677adc381c9d8'),
        ('zlib-LICENSE.txt', 'https://raw.githubusercontent.com/madler/zlib/v1.3.1/LICENSE',
         '845efc77857d485d91fb3e0b884aaa929368c717ae8186b66fe1ed2495753243'),
    ]:
        shutil.copy2(fetch(filename, url, digest), appdir / 'native-licenses' / filename)
    (sources / 'AppImage-runtime.json').write_text(json.dumps({
        'commit': RUNTIME_COMMIT, 'runtime_url': runtime_url, 'runtime_sha256': RUNTIME_SHA,
        'source_url': source_url, 'source_sha256': SOURCE_SHA,
        'rebuild': 'See scripts/docker/build-with-docker.sh in the included source. The runtime can be replaced independently of WebDrop.',
    }, indent=2) + '\n')
    image = work / 'payload.squashfs'
    run('mksquashfs', appdir, image, '-noappend', '-comp', 'zstd', '-no-xattrs', '-all-root', '-no-progress')
    output = DIST / 'WebDrop-x86_64.AppImage'
    with output.open('wb') as dest:
        for p in [runtime, image]:
            with p.open('rb') as src:
                shutil.copyfileobj(src, dest)
    output.chmod(0o755)
    assert output.read_bytes()[8:11] == b'AI\x02', 'Missing AppImage type 2 signature'
    return output


if __name__ == '__main__':
    DIST.mkdir(exist_ok=True)
    WORK.mkdir(parents=True, exist_ok=True)
    if len(sys.argv) != 2 or sys.argv[1] not in ('macos', 'windows', 'linux'):
        raise SystemExit('Usage: package-installers.py macos|windows|linux')
    output = {'macos': macos, 'windows': windows, 'linux': linux}[sys.argv[1]]()
    print(f'{output.name}: {output.stat().st_size:,} bytes')
