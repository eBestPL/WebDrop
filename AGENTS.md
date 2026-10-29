# Project preferences

- Build and package macOS by default.
- Do not build or refresh Windows/Linux distributions unless the user explicitly asks, or asks for Git commit commands. Keep existing cross-platform packages unchanged during ordinary development.
- README screenshots must show the real application without the automation cursor or its glow. Use demonstration files, not private user files.

- Distribute one downloadable file per OS: macOS `.pkg`, Windows `-Setup.exe`, Linux `.AppImage`. Do not wrap downloads in ZIP/TAR archives.
