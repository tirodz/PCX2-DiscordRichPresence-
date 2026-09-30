# PCSX2 Discord Rich Presence

A small Windows helper that connects PCSX2 to Discord Rich Presence through PCSX2's native PINE interface.

The goal is to make it feel like part of PCSX2: start PCSX2, and the Discord activity follows it without having to launch an RPC program every time.

## What it does

- Shows an idle PCSX2 activity while the emulator is sitting at its main menu
- Detects the PS2 system menu/BIOS state
- Reads the game title, serial, CRC and version through PINE
- Finds PS2 cover art from the serial and uses it in Discord
- Shows playing and paused states
- Keeps the elapsed timer tied to the game session instead of resetting on pause/resume
- Caches downloaded covers locally
- Reconnects cleanly when Discord or PCSX2 goes away
- Can be installed once to start automatically with Windows
- Runs without an extra visible helper window

## How it works

PCX2-DiscordRichPresence does not scrape the PCSX2 window or inject code into the emulator.

The helper talks to the PINE TCP server on `127.0.0.1:28011`, turns the emulator's status and metadata into a small state machine, resolves cover art from the game serial, and publishes the result through Discord IPC.

When PINE is unavailable, the Discord activity is cleared. The helper stays in the background so the next PCSX2 launch is picked up automatically.

## Setup

### 1. Enable PINE in PCSX2

Enable the PINE server in PCSX2. The default Windows slot is `28011`.

### 2. Create a Discord application

Create a Discord application and copy its Application ID (Client ID). This is an application identifier, not your Discord account token.

Put it in `config.toml` next to the executable:

```toml
[pine]
host = "127.0.0.1"
port = 28011

[discord]
client_id = "YOUR_DISCORD_APPLICATION_ID"

poll_seconds = 2
retry_seconds = 3
```

### 3. Install automatic startup

Run the executable once with:

```text
pcsx2-discord-rich-presence.exe --install
```

That adds a per-user Windows startup entry. No administrator privileges are needed.

Useful commands:

```text
--install      enable automatic startup
--uninstall    remove automatic startup
--status       check the startup entry
--help         show the available commands
```

After that, the normal flow is simply:

**PCSX2 starts → PINE becomes available → Discord presence appears → game/paused/menu state follows → PCSX2 closes → presence clears.**

## Development

Use a current stable Rust toolchain:

```text
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

GitHub Actions runs the formatting, tests, Clippy and Windows release build on pushes and pull requests.

## References

- PCSX2 PINE: https://github.com/PCSX2/pcsx2/blob/master/pcsx2/PINE.cpp
- PINE reference client: https://github.com/GovanifY/pine
- PS2 cover database: https://github.com/xlenore/ps2-covers
- Extreme-InfiniTV: https://github.com/infinitel8p/Extreme-InfiniTV
