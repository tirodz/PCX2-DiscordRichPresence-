# Maintainer Guide

## Purpose

This repository provides a Windows companion for PCSX2 that publishes a richer Discord Rich Presence activity than the emulator's basic built-in presence.

The implementation is intentionally external for now. A future goal is to evaluate whether the useful parts can become a native PCSX2 feature.

## Project standards

- Preserve working behavior and avoid throwaway prototypes.
- Keep normal operation silent and failures recoverable.
- Keep the installer and uninstaller clean and user-level.
- Keep user-facing copy, commits, changelog entries, and release notes natural and concise.
- Keep temporary experiments, debug dumps, and unrelated binaries out of the repository.
- Never patch PCSX2 binaries or interfere with Discord globally.
- PCSX2 configuration changes must be narrow, reversible, and restored safely on uninstall.
- Do not claim live PCSX2 or Discord testing unless it was actually performed.

## Runtime architecture

The helper uses PCSX2's PINE IPC on localhost.

Normal runtime flow:

PCSX2 -> PINE -> state/metadata -> cover lookup -> Discord Rich Presence

Default PINE endpoint: 127.0.0.1:28011.

States currently represented:

- Offline: PCSX2/PINE unavailable; activity cleared.
- Idle: PCSX2 is open with no active PS2 VM; show the main-menu state.
- BIOS: a PS2 VM is active but title/serial metadata is unavailable; show the PlayStation 2 system-menu state.
- Game: title/serial metadata is available; show the game title, cover, play state, and session timer.
- Paused: preserve the game session start time and change only the state text.

## PCSX2 configuration ownership

During setup the application can:

- enable PINE;
- preserve the user's previous PINE settings;
- disable PCSX2's optional built-in Discord presence;
- preserve the previous Discord setting;
- restore the saved settings during uninstall when the files were not independently changed.

Do not broaden this to unrelated PCSX2 settings.

## Discord

Public releases are intended to use one project-owned Discord application ID supplied at build time through PCSX2_DISCORD_CLIENT_ID.

Never ship a public release that silently falls back to asking every user to create a Discord Developer Application.

Never put a Discord token, password, or other private credential in the repository.

The stable BIOS/system-menu artwork lives at assets/bios/ps2-bios.png. Do not add a Discord logo to that artwork or replace it with a generated substitute.

Dynamic game covers come from the PS2 serial database and are transformed into a Discord-compatible square image before publication.

## Setup UX

The normal public user journey should be:

Download -> Install -> Start PCSX2 -> Discord Presence

PCSX2 detection should be automatic whenever practical. PINE and application-ID fields may remain in settings for advanced troubleshooting and self-built binaries, but they should not be part of the normal public path when project configuration is available.

## Cover presentation

Discord's large Rich Presence image area is square while PS2 box art is commonly portrait.

The current target is a 1024px smart-square rendition:

- never stretch the artwork;
- avoid unnecessary black side borders;
- do not blindly crop away important cover content;
- preserve recognizable cover composition;
- test multiple regions and aspect ratios.

## State presentation

Keep the presence concise. Current examples:

- 🏠 At the Main Menu
- ⚙ System Menu
- 🎮 Playing on PCSX2
- ⏸ Paused on PCSX2

Use emoji sparingly and only when it improves scanning.

## Release process

Before publishing a public release:

1. Run formatting, tests, Clippy, and release build.
2. Build the Windows installer.
3. Build the portable ZIP.
4. Generate SHA-256 checksums.
5. Verify all artifacts exist.
6. Verify the release workflow succeeds.
7. Verify the GitHub Release contains the installer, ZIP, and checksum file.

Never publish an empty release.

## Current v0.1.1 focus

- Reliable project PS2 BIOS artwork delivery.
- Better portrait game-cover framing.
- Automatic PINE configuration.
- Automatic PCSX2 detection.
- A simpler first-run flow.
- Clean and deterministic release/test infrastructure.

## Long-term roadmap

### Phase 1 — polished companion

Make the current Windows application reliable, quiet, easy to install, and visually polished.

### Phase 2 — zero-configuration experience

Hide implementation details such as PINE and the Discord application ID from normal users. Detect PCSX2 automatically and configure the integration safely.

### Phase 3 — native PCSX2 evaluation

After the companion is mature, investigate whether richer Discord presence belongs directly in PCSX2 and discuss that direction with the upstream project.

Do not submit upstream changes automatically. Prepare a clean implementation, documentation, tests, and rationale first.