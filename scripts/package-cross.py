"""Package cross-compiled binaries with their verified native library closure."""
import hashlib, json, re, shutil, subprocess, struct, sys, tarfile, zipfile
from pathlib import Path
root = Path(__file__).resolve().parent.parent
platform = sys.argv[1]
assert platform in ('linux', 'windows')
windows = platform == 'windows'
name = 'WebDrop-Windows-x64' if windows else 'WebDrop-Linux-x64'
staging = root / 'artifacts' / 'packages' / name
if staging.exists():
    shutil.rmtree(staging)
staging.mkdir(parents=True)
target = 'x86_64-pc-windows-gnu' if windows else 'x86_64-unknown-linux-gnu'
source = root / 'target' / target / 'release' / ('image-optimizer.exe' if windows else 'image-optimizer')
executable = staging / ('WebDrop.exe' if windows else 'WebDrop')
shutil.copy2(source, executable)
if not windows:
    # DT_RPATH is inherited by codec dependencies; DT_RUNPATH only resolves
    # direct imports. Zig emits RUNPATH even with --disable-new-dtags.
    data = bytearray(executable.read_bytes())
    assert data[:6] == b'\x7fELF\x02\x01', 'Expected ELF64 little-endian'
    phoff = struct.unpack_from('<Q', data, 32)[0]
    phsize, phcount = struct.unpack_from('<HH', data, 54)
    changed = False
    for i in range(phcount):
        offset = phoff + i * phsize
        if struct.unpack_from('<I', data, offset)[0] != 2: continue  # PT_DYNAMIC
        start = struct.unpack_from('<Q', data, offset + 8)[0]
        size = struct.unpack_from('<Q', data, offset + 32)[0]
        for entry in range(start, start + size, 16):
            tag = struct.unpack_from('<Q', data, entry)[0]
            if tag == 29:  # DT_RUNPATH -> DT_RPATH; string and offsets unchanged
                struct.pack_into('<Q', data, entry, 15)
                changed = True
            elif tag == 15:
                changed = True
    assert changed, 'Missing bundled-library search path'
    executable.write_bytes(data)
libroot = root / '.tools/cross' / ('windows/mingw64/bin' if windows else 'linux/usr/lib/x86_64-linux-gnu')
libdest = staging if windows else staging / 'lib'
libdest.mkdir(exist_ok=True)
objdump = subprocess.check_output(['xcrun', '--find', 'llvm-objdump'], text=True).strip()
system_windows = set('advapi32 bcrypt bcryptprimitives combase uiautomationcore cfgmgr32 comctl32 comdlg32 crypt32 d2d1 d3d11 d3d12 d3dcompiler_47 dbghelp dcomp dwmapi dwrite dxgi gdi32 imm32 iphlpapi kernel32 kernelbase msvcrt ncrypt ntdll ole32 oleaut32 opengl32 powrprof propsys psapi rpcrt4 secur32 setupapi shcore shell32 shlwapi ucrtbase user32 userenv usp10 uxtheme version winhttp winmm winspool wintrust ws2_32 wtsapi32'.split())
system_linux = set('libc.so.6 libm.so.6 libdl.so.2 libpthread.so.0 librt.so.1 libresolv.so.2 ld-linux-x86-64.so.2'.split())
available = {p.name.lower(): p for p in libroot.iterdir() if p.is_file()}
copied, system = {}, set()
def visit(path):
    info = subprocess.check_output([objdump, '-p', str(path)], text=True)
    pattern = r'DLL Name:\s*(\S+)' if windows else r'^\s*NEEDED\s+(\S+)'
    for dependency in re.findall(pattern, info, re.M):
        key = dependency.lower()
        builtin = ((key[:-4] if key.endswith('.dll') else key) in system_windows or key.startswith(('api-ms-', 'ext-ms-'))) if windows else dependency in system_linux
        if builtin:
            system.add(dependency)
            continue
        if key in copied:
            continue
        if key not in available:
            raise RuntimeError(f'Unresolved dependency {dependency} from {path.name}')
        library = available[key]
        copied[key] = dependency
        shutil.copy2(library.resolve(), libdest / dependency)
        visit(library)
visit(executable)
for filename in ('LICENSE', 'README.md', 'README.pl.md'):
    shutil.copy2(root / filename, staging / filename)
shutil.copytree(root / 'docs/screenshots', staging / 'docs/screenshots')
shutil.copy2(root / 'artifacts/THIRD-PARTY-NOTICES.txt', staging / 'THIRD-PARTY-NOTICES.txt')
licenses = staging / 'native-licenses'
licenses.mkdir()
zigroot = root / '.tools/cross/zig-aarch64-macos-0.15.2'
for source_name, output_name in [('LICENSE', 'zig.txt'), ('lib/libunwind/LICENSE.TXT', 'llvm-libunwind.txt'), ('lib/libc/mingw/COPYING', 'mingw-runtime.txt')]:
    shutil.copy2(zigroot / source_name, licenses / output_name)
license_root = root / '.tools/cross' / ('windows/mingw64/share/licenses' if windows else 'linux/usr/share/doc')
for package in license_root.iterdir():
    if not package.is_dir(): continue
    if windows:
        shutil.copytree(package, licenses / package.name)
    elif (package / 'copyright').is_file():
        shutil.copy2(package / 'copyright', licenses / (package.name + '.txt'))
shutil.copy2(root / '.tools/cross/packages.json', staging / 'native-packages.json')
(staging / 'BUILD-INFO.json').write_text(json.dumps({'version':'0.9.2','target':target,'runtime_tested':False,'bundled_libraries':sorted(copied.values()),'system_libraries':sorted(system)}, indent=2)+'\n')
start = 'Extract this entire ZIP, then open WebDrop.exe. Keep the DLL files beside the app.\nWindows 10/11 x64.\n' if windows else 'Extract the archive, then launch the WebDrop executable.\nRequires Linux x64, glibc 2.41+, a graphical desktop with OpenGL, X11 or Wayland,\nlibxkbcommon and xdg-desktop-portal with a desktop backend for file dialogs.\n'
(staging / 'START-HERE.txt').write_text('WebDrop 0.9.2\n\n'+start+'\nPreview build: compiled on macOS; not yet run on this operating system.\nPNG, JPG, WebP, AVIF, JXL and SVG are supported. AI/PDF, TIFF and HEIC import is macOS-only.\n\nPL: '+('Rozpakuj całe archiwum, następnie otwórz WebDrop.exe. Biblioteki DLL muszą pozostać obok aplikacji.' if windows else 'Rozpakuj archiwum i uruchom plik WebDrop. Wymagany Linux x64 z glibc 2.41+, pulpitem graficznym i OpenGL.')+'\nWersja testowa: skompilowana na macOS; nie była jeszcze uruchamiana na tym systemie.\nImport AI/PDF, TIFF i HEIC jest dostępny tylko na macOS.\n')
dist = root / 'dist'
dist.mkdir(exist_ok=True)
if windows:
    archive = dist / (name + '.zip')
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as output:
        for path in sorted(staging.rglob('*')):
            if path.is_file(): output.write(path, path.relative_to(staging.parent))
else:
    executable.chmod(0o755)
    archive = dist / (name + '.tar.gz')
    with tarfile.open(archive, 'w:gz') as output:
        output.add(staging, arcname=name)
print(f'{archive.name}: {archive.stat().st_size:,} bytes; {len(copied)} bundled libraries')
