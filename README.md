# PCSX2 Discord Rich Presence

A small Windows companion that shows your PCSX2 session in Discord: which game
you're playing, how long you've been playing it, and whether you're paused -
with the actual PS2 cover art.

It talks to PCSX2 through PINE, the IPC interface built into the emulator. No
window scraping, no memory reading, and no PCSX2 binary patching.

## What it looks like

| PCSX2 state | Discord activity |
| --- | --- |
| PCSX2 open, no game | **PCSX2** - At the Main Menu |
| PS2 system menu | **PlayStation 2** - System Menu |
| Game running | **Game title** - Playing on PCSX2, cover art, elapsed time |
| Game paused | **Game title** - Paused on PCSX2, timer keeps counting |
| PCSX2 closed | activity cleared |

The elapsed timer belongs to the game session: pausing and resuming does not
reset it, closing the game does.

## Requirements

- Windows 10 or 11 (64-bit)
- PCSX2 2.x (any recent Qt build, installed or portable)
- The Discord desktop app
- A Discord application ID only for self-built binaries when the project ID is not supplied

## Installation

Download the latest release from the
[Releases](https://github.com/tirodz/PCX2-DiscordRichPresence-/releases) page:

- **`PCSX2-DiscordRichPresence-v0.1.1-Setup.exe`** - installer (recommended)
- **`PCSX2-DiscordRichPresence-v0.1.1-windows-x64.zip`** - portable build, just
  unzip and run

The installer does not need administrator rights. It puts the app in your user
profile, adds a Start Menu entry, and offers to run the setup wizard when it
finishes. Public builds can carry the project's Discord application ID so normal
users do not have to visit the Discord Developer Portal. During setup, the app also disables PCSX2's own Discord presence so
the custom activity is the only one being published. The original PCSX2 setting
is restored when you uninstall.

## First-run setup

The public release is designed to keep setup simple:

1. **PCSX2 location** - the app tries to detect PCSX2 automatically. If it cannot find it, browse to `pcsx2-qt.exe` once.
2. **PCSX2 integration** - the app configures the required PINE settings automatically and preserves the previous PCSX2 values for uninstall.
3. **Discord** - public builds use the project's application ID automatically.
4. **Confirm** - choose whether the helper should start with Windows and start right away.

Normal users do not need to know the PINE port, open PCSX2's Advanced settings, or create a Discord Developer Application. Manual PINE and Discord fields remain available in the settings window for development and troubleshooting.

## Everyday use

There is nothing to do. The helper starts with Windows, sits quietly in the
background (no window, no console), and waits for PCSX2:

- Start PCSX2 -> Discord shows **PCSX2 - At the Main Menu**
- Boot a game -> title, cover art and a running timer appear
- Pause -> status switches to *Paused on PCSX2*, the timer keeps counting
- Close the game -> back to the main menu status
- Quit PCSX2 -> the Discord activity clears

If Discord isn't running, the helper simply waits and publishes once Discord
is back. If PCSX2 was already running when you finish setup, restart PCSX2 so
its updated Discord setting is picked up.

## Cover artwork

Covers come from the [xlenore/ps2-covers](https://github.com/xlenore/ps2-covers)
database, matched by the game's serial (SLUS/SCES/SLPS/...). Because Discord renders a Rich Presence large image as a square, covers are
served as a **1024 x 1024 smart square crop** so portrait box art fills the
card more naturally without stretching. Covers are cached locally, serials without a
cover are remembered instead of re-fetched, and a missing cover never breaks
the text presence - the PCSX2 logo is used as a fallback.

## Validation status

The project has automated PINE, state, cover-cache, timestamp, configuration, and packaging checks. A live Windows session with PCSX2 and Discord is still required to visually verify the PS2 BIOS artwork and game-cover framing in the Rich Presence card.

Public release builds use the project's Discord application ID supplied through the `PCSX2_DISCORD_CLIENT_ID` GitHub Actions secret. The `ps2-bios` image key must also exist in that Discord application's registered Rich Presence assets; a repository PNG or public image URL does not automatically register an asset with Discord. If the registered asset is missing or named differently, Discord may show a question-mark placeholder. Verify the asset in the Discord Developer Portal before publishing a release.

## Troubleshooting

The helper writes `helper.log` next to the executable - look there first.

- **Nothing shows in Discord.** Make sure the Discord desktop app is running
  (the browser version can't show rich presence), and that *Activity Privacy ->
  Share your detected activities* is enabled in Discord's settings.
- **PINE is not detected.** Restart PCSX2 after a fresh setup if it was already open.
  Advanced PINE host/slot fields remain available in Settings for troubleshooting.
- **Wrong or missing cover.** The cover database is keyed by serial; some
  releases (homebrew, prototypes, some betas) have no entry. The game title
  and timer still work.
- **The status is stuck after quitting PCSX2.** It should clear within a few
  seconds. If it doesn't, check `helper.log` and restart the helper from the
  settings window (*Save and restart helper*).
- **Command line.** `--status` shows whether startup and the helper are
  active; `--install` / `--uninstall` manage the Windows startup entry;
  `--background` is what the startup entry runs.

## Uninstalling

Use *Add or Remove Programs* or the Start Menu uninstall entry. This stops the
background helper, removes the startup registration, and removes the app's
files and configuration. The app attempts to restore the PCSX2 PINE and built-in
Discord settings it changed, while preserving later manual edits to PCSX2's
configuration. Your PCSX2 installation itself is not removed.

## Building from source

You need a current stable Rust toolchain. Everything else is fetched by
Cargo:

```text
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

The Windows executable lands in `target\release`. To build the installer, run
Inno Setup (`ISCC.exe installer.iss`) after the release build.

GitHub Actions runs the checks and build on every push, and publishes the
installer, the portable ZIP and SHA-256 checksums when a `v*` tag is pushed.

## Credits

- [PCSX2](https://github.com/PCSX2/pcsx2) and its PINE interface
- [GovanifY/pine](https://github.com/GovanifY/pine), the PINE reference client
- [xlenore/ps2-covers](https://github.com/xlenore/ps2-covers) for the cover art
