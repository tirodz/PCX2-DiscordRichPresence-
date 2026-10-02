# Changelog

## Unreleased

## 0.1.1 - 2026-10-02

- Use the project PlayStation 2 artwork for the BIOS/system-menu presence.
- Deliver BIOS artwork through a Discord-compatible public media path, with an optional registered asset-key override.
- Improve portrait game-cover framing with a 1024px smart square crop.
- Configure PCSX2 PINE automatically during setup and restore the previous settings on uninstall.
- Detect common PCSX2 installation locations automatically.
- Remove the normal PINE configuration step from first-run setup.
- Keep the project Discord application ID as the public-build path, with manual configuration available for self-built binaries.
- Add deterministic PINE, cover-cache, configuration, and Discord regression coverage.
- Add a Windows CI runtime guard and verified release packaging.
- Add the MIT license and structured project changelog.

## 0.1.0 - 2026-10-01

- Initial public Windows release.
- PINE-based PCSX2 state detection.
- Game title and serial detection with PS2 cover artwork.
- Playing/paused session states with persistent elapsed time.
- PCSX2 menu and BIOS/system-menu presence.
- Windows installer and portable ZIP.
- Optional suppression of PCSX2's built-in Discord presence while the companion is active.
