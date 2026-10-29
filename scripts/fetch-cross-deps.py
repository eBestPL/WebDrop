"""Fetch verified Debian/MSYS2 native packages into the local cross-build cache."""
import hashlib, io, json, lzma, re, subprocess, tarfile, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / '.tools/cross'
ROOT.mkdir(parents=True, exist_ok=True)
manifest = []

def download(url, name, digest=None):
    dest = ROOT / 'downloads' / name
    dest.parent.mkdir(parents=True, exist_ok=True)
    if not dest.exists():
        print('Downloading', name, flush=True)
        urllib.request.urlretrieve(url, dest)
    actual = hashlib.sha256(dest.read_bytes()).hexdigest()
    if digest and actual != digest:
        raise RuntimeError('Checksum mismatch: ' + name)
    manifest.append({'url': url, 'sha256': actual, 'file': name})
    return dest

def records(text):
    for block in text.split('\n\n'):
        record = {}
        for line in block.splitlines():
            if ': ' in line and not line.startswith(' '):
                key, value = line.split(': ', 1)
                record[key] = value
        if 'Package' in record:
            yield record

# Pinned host compiler, verified against the official Zig release manifest.
zig = download('https://ziglang.org/download/0.15.2/zig-aarch64-macos-0.15.2.tar.xz',
               'zig-aarch64-macos-0.15.2.tar.xz',
               '3cc2bab367e185cdfb27501c4b30b1b0653c28d9f73df8dc91488e66ece5fa6b')
if not (ROOT / 'zig-aarch64-macos-0.15.2/zig').is_file():
    subprocess.run(['tar', '-xf', str(zig), '-C', str(ROOT)], check=True)

# Debian 13 headers and runtime library closure.
base = 'https://deb.debian.org/debian/'
index = download(base + 'dists/trixie/main/binary-amd64/Packages.xz', 'debian-trixie-Packages.xz')
pkgs = {p['Package']: p for p in records(lzma.decompress(index.read_bytes()).decode())}
linux = ROOT / 'linux'
linux.mkdir(exist_ok=True)
seen = set()
def deb(name):
    if name in seen: return
    seen.add(name)
    if name not in pkgs: raise RuntimeError('Missing Debian package: ' + name)
    p = pkgs[name]
    archive = download(base + p['Filename'], Path(p['Filename']).name, p['SHA256'])
    # ar extraction runs in a separate directory to avoid collisions between packages.
    work = ROOT / 'unpack' / name
    work.mkdir(parents=True, exist_ok=True)
    subprocess.run(['ar', '-x', str(archive)], cwd=work, check=True)
    data = next(work.glob('data.tar.*'))
    subprocess.run(['tar', '-xf', str(data), '-C', str(linux)], check=True)
    for field in ('Depends', 'Pre-Depends'):
        for group in p.get(field, '').split(','):
            if group.strip():
                dep = re.split(r'[ (:\[]', group.strip())[0]
                if dep in pkgs: deb(dep)
for name in ['libavif-dev', 'libjxl-dev']:
    deb(name)

# MinGW runtime libraries and headers; all archives are verified against the repository database.
base = 'https://mirror.msys2.org/mingw/mingw64/'
index = download(base + 'mingw64.db', 'mingw64.db')
pkgs = {}
db_dir = ROOT / 'msys-db'
db_dir.mkdir(exist_ok=True)
subprocess.run(['tar', '-xf', str(index), '-C', str(db_dir)], check=True)
for desc in db_dir.glob('*/desc'):
    parts = desc.read_text().strip().split('\n\n')
    p = {block.splitlines()[0].strip('%'): block.splitlines()[1:] for block in parts}
    pkgs[p['NAME'][0]] = p
providers = {re.split(r'[<>=]', alias)[0]: name for name, package in pkgs.items() for alias in package.get('PROVIDES', [])}
windows = ROOT / 'windows'
windows.mkdir(exist_ok=True)
seen = set()
def msys(name):
    name = name if name in pkgs else providers.get(name, name)
    if name in seen: return
    seen.add(name)
    p = pkgs[name]
    filename = p['FILENAME'][0]
    archive = download(base + filename, filename, p['SHA256SUM'][0])
    subprocess.run(['tar', '-xf', str(archive), '-C', str(windows)], check=True)
    for dep in p.get('DEPENDS', []):
        msys(re.split(r'[<>=]', dep)[0])
for name in ['mingw-w64-x86_64-libavif', 'mingw-w64-x86_64-libjxl', 'mingw-w64-x86_64-winpthreads']:
    msys(name)
(ROOT / 'packages.json').write_text(json.dumps(manifest, indent=2) + '\n')
print('Verified native dependencies ready for both targets.', flush=True)
