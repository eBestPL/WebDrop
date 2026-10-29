"""Collect bundled dependency license notices from Cargo's locked metadata."""
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(
    [str(root / "scripts/cargo.sh"), "metadata", "--format-version", "1", "--locked", "--offline"], cwd=root
))
sections = ["WebDrop Image Optimizer — third-party notices\nResolved dependencies, including platform-specific build dependencies.\n"]
for package in sorted(metadata["packages"], key=lambda p: p["name"]):
    if package["name"] == "image-optimizer":
        continue
    base = Path(package["manifest_path"]).parent
    sections.append(f"\n{'=' * 72}\n{package['name']} {package['version']}\nLicense: {package.get('license') or 'See license text'}\n{package.get('repository') or ''}\n")
    candidates = list(base.iterdir())
    vendor = base / "vendor"
    if vendor.exists():
        candidates.extend(vendor.rglob("*"))
    for path in sorted(candidates):
        if path.is_file() and path.name.upper().startswith(("LICENSE", "COPYING", "COPYRIGHT", "NOTICE")):
            sections.append(f"\n--- {path.relative_to(base)} ---\n{path.read_text(errors='replace')}\n")
output = root / "artifacts/THIRD-PARTY-NOTICES.txt"
output.parent.mkdir(parents=True, exist_ok=True)
sections.append((root / "artifacts/NATIVE-CODEC-NOTICES.txt").read_text())
ghostscript = root / "artifacts/GHOSTSCRIPT-NOTICES.txt"
if ghostscript.exists():
    sections.append(ghostscript.read_text())
mupdf = root / "artifacts/MUPDF-NOTICES.txt"
if mupdf.exists():
    sections.append(mupdf.read_text())
output.write_text("\n".join(sections))
