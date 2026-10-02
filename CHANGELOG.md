# Changelog

## Unreleased

## 0.1.1 - 2026-10-02

- Use the project PlayStation 2 artwork for the BIOS/system-menu presence.
- Improve portrait game-cover framing with a smart square crop.
- Configure PCSX2 PINE automatically during setup and restore the previous settings on uninstall.
- Detect common PCSX2 installation locations automatically.
- Shorten the normal first-run flow by removing manual PINE configuration from the main path.
- Add a build-time hook for a future project-owned Discord application ID.

- Refine the PlayStation 2 BIOS/system-menu artwork and Discord presentation.
- Investigate improved cover framing so portrait PS2 covers use Discord's square image area more effectively.
- Longer-term direction: reduce setup to a simple install-and-run experience, hiding PINE and Discord application configuration from normal users where technically possible.
- Longer-term direction: evaluate a native PCSX2 integration instead of requiring a separate companion.

## 0.1.0 - 2026-10-01

- Initial public Windows release.
- PINE-based PCSX2 state detection.
- Game title and serial detection with PS2 cover artwork.
- Playing/paused session states with persistent elapsed time.
- PCSX2 menu and BIOS/system-menu presence.
- Windows installer and portable ZIP.
- Optional suppression of PCSX2's built-in Discord presence while the companion is active.
