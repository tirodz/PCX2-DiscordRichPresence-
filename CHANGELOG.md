# Changelog

## Unreleased

- Fix BIOS artwork delivery by using a Discord-compatible square media path and an optional registered asset-key override.
- Keep the supplied PS2 BIOS artwork under `assets/bios/ps2-bios.png`.
- Simplify the normal setup path so PINE is configured automatically.
- Keep the project Discord application ID as the public-build path, with manual configuration available for self-builds.
- Keep the cover pipeline at 1024px smart-square output and document the current architecture accurately.

## 0.1.1 - 2026-10-02

- Use the project PlayStation 2 artwork for the BIOS/system-menu presence.
- Improve portrait game-cover framing with a smart square crop.
- Configure PCSX2 PINE automatically during setup and restore the previous settings on uninstall.
- Detect common PCSX2 installation locations automatically.
- Shorten the normal first-run flow by removing manual PINE configuration from the main path.
- Add a build-time hook for a project-owned Discord application ID.
- Add the MIT license and structured project changelog.

## 0.1.0 - 2026-10-01

- Initial public Windows release.
- PINE-based PCSX2 state detection.
- Game title and serial detection with PS2 cover artwork.
- Playing/paused session states with persistent elapsed time.
- PCSX2 menu and BIOS/system-menu presence.
- Windows installer and portable ZIP.
- Optional suppression of PCSX2's built-in Discord presence while the companion is active.
