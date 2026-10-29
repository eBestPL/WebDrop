# Release packages

Downloads are single files: macOS `.pkg`, Windows `-Setup.exe`, Linux `.AppImage`.

On Apple Silicon macOS, install packaging tools once:

```sh
brew install makensis squashfs
```

After building the app, `./scripts/package-release.sh` refreshes only the Mac installer and checksums. To explicitly refresh every platform:

```sh
./scripts/build-cross.sh linux
./scripts/build-cross.sh windows
./scripts/bundle.sh
./scripts/package-release.sh --all
```

For documentation-only changes, reuse existing builds and run the package script with `--all`.

- **macOS:** standard Installer package, installs to `/Applications/WebDrop.app`, requires macOS 15+ on Apple Silicon. No post-install scripts. App signature is checked before packaging; the installer is not Developer ID signed or notarized.
- **Windows:** NSIS installer for Windows 10+ x64, per-user installation with bundled DLLs, Start menu shortcut and uninstaller. No administrator rights requested. Uninstall removes only listed app files, leaving preferences and user images intact. The installer is unsigned.
- **Linux:** type 2 AppImage with a pinned, SHA-256-verified official runtime and bundled native libraries. Requires the desktop dependencies listed in README and FUSE. Without FUSE, `./WebDrop-x86_64.AppImage --appimage-extract-and-run` temporarily extracts and launches automatically. Mark the downloaded file executable first.

The AppImage runtime and source are cached in `.tools/appimage`. Downloads fail on checksum mismatch; update pinned versions deliberately. Runtime licenses, source, patches and build instructions are included in the AppImage. The runtime can be rebuilt and replaced independently of WebDrop.

Packaging checks do not replace install/launch/uninstall tests on Windows and Linux. These downloads remain preview builds until tested on those systems.
