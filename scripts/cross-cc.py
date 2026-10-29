"""Adapt Rust/cc target names to Zig target names without shell argument splitting."""
import os
from pathlib import Path
import sys
root = Path(__file__).resolve().parent.parent
zig = root / '.tools/cross/zig-aarch64-macos-0.15.2/zig'
target = {'linux': 'x86_64-linux-gnu.2.41', 'windows': 'x86_64-windows-gnu'}[os.environ['WEBDROP_CROSS_TARGET']]
args = []
skip = False
for arg in sys.argv[1:]:
    if skip:
        skip = False
    elif arg in ('--target', '-target'):
        skip = True
    elif not arg.startswith('--target='):
        args.append(arg)
# Work around Zig 0.15 LLVM AVX-512 target-attribute incompatibility.
# Keep portable SSE/PCLMUL paths; do not require AVX-512 on users' CPUs.
if any('libdeflate/' in arg for arg in args):
    args.extend(['-DLIBDEFLATE_ASSEMBLER_DOES_NOT_SUPPORT_VPCLMULQDQ', '-DLIBDEFLATE_ASSEMBLER_DOES_NOT_SUPPORT_AVX512VNNI'])
if os.environ['WEBDROP_CROSS_TARGET'] == 'windows':
    # Zig's forced dynamic search does not consider GNU import archives.
    directories = [Path(arg[2:]) for arg in args if arg.startswith('-L') and len(arg) > 2]
    directories.extend(Path(args[i + 1]) for i, arg in enumerate(args[:-1]) if arg == '-L')
    rewritten = []
    for arg in args:
        if arg in ('-Wl,-Bdynamic', '-Wl,-Bstatic', '-lmsvcrt', '-lmingw32', '-lmingwex', '-lgcc', '-lgcc_eh'):
            continue
        if arg.startswith('-l'):
            name = arg[2:]
            found = next((directory / filename for directory in directories
                          for filename in ((name[1:],) if name.startswith(':') else ('lib' + name + '.dll.a', 'lib' + name + '.a'))
                          if (directory / filename).is_file()), None)
            if found is not None:
                arg = str(found)
        rewritten.append(arg)
    if '-nodefaultlibs' in rewritten:
        rewritten.append('-lunwind')
    args = rewritten
os.execv(zig, [str(zig), 'cc', '-target', target, *args])
