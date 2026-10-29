# WebDrop — Image Optimizer

[English](README.md) · [Polski](README.pl.md)

Optimize images and convert whole folders for the web. Free, local, no uploads.

## ⬇️ Download

| System | Download | Install |
| --- | --- | --- |
| 🍎 macOS 15+ · Apple Silicon | **[Download for Mac](dist/WebDrop-macOS-arm64.zip?raw=true)** | Unzip and move WebDrop.app to Applications |
| 🪟 Windows 10/11 · x64 | **[Download for Windows](dist/WebDrop-Windows-x64.zip?raw=true)** | Extract the entire ZIP, then open WebDrop.exe |
| 🐧 Linux · x64 | **[Download for Linux](dist/WebDrop-Linux-x64.tar.gz?raw=true)** | Extract and launch WebDrop |

**Version 0.9.4.** Windows/Linux are preview builds, not yet tested on those systems. Linux requires glibc 2.41+, OpenGL, X11/Wayland, libxkbcommon and a desktop portal. macOS is not notarized; Windows is unsigned.

[SHA-256 checksums](dist/SHA256SUMS.txt)

![WebDrop in dark mode — Hybrid batch conversion](docs/screenshots/batch-dark-en.jpg)

## How to use

1. Drop images or folders into the window. Subfolders are included.
2. Choose your settings, then click **Start conversion**.
3. Find the results in **Downloads**, or choose the source folder or a custom location.

Defaults: **WebP · up to 1500 px wide · quality 82**. Originals are never overwritten. Use **Re-run** to process the same files with different settings.

## Features

- **Formats:** PNG, JPG, WebP, AI, PDF, EPS, TIFF, HEIC, SVG, AVIF and JPEG XL.*
- **Resize:** 1500, 1000, 700 or 500 px wide, keeping proportions; no upscaling.
- **Logo preset:** lossless WebP with **trim** — removes transparent or solid-color outer margins, not the background inside a logo.
- **Hybrid mode:** AI/PDF/EPS → SVG; raster images → WebP.
- **SVG stays SVG:** no optimization or conversion; optional canvas trimming only.
- **Privacy:** raster outputs always have source metadata removed, including EXIF and GPS. Existing SVG contents are preserved.
- **Interface:** English/Polish, light/dark themes, remembered output folder.

*AI must be PDF-compatible. TIFF/HEIC and AI/PDF rasterization are macOS-only. Windows/Linux need MuPDF (`mutool` in PATH) for AI/PDF → SVG, plus Ghostscript for EPS. Both tools are bundled on macOS.*

![WebDrop in light mode — Logo preset with trimming](docs/screenshots/logo-light-en.jpg)

<details>
<summary>Build from source (macOS)</summary>

Requires Rust 1.95+, Xcode Command Line Tools and Python 3. Built with Rust and egui/eframe.

```sh
brew install libavif jpeg-xl mupdf-tools ghostscript
./scripts/bundle.sh
open dist/WebDrop.app
```

</details>

## Author & license

**Jędrzej Czarnecki** · [info@ebest.pl](mailto:info@ebest.pl) · [ebest.pl](https://ebest.pl)

Free and open source under [GPL-3.0-or-later](LICENSE). Third-party licenses are included in the app.
