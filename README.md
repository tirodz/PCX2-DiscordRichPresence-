# PCSX2 Discord Rich Presence

Automatic Discord Rich Presence for PCSX2.

The project is designed around PCSX2's native PINE IPC interface rather than window-title scraping or emulator injection.

## Goals

- PCSX2 idle/menu presence
- PS2 BIOS/system-menu presence
- Serial-aware game identification
- Game title and PS2 cover artwork
- Playing and paused states
- Accurate elapsed-play timestamps
- Local cover caching
- Discord reconnect handling
- One-time setup with no manual RPC launch for every session
- Lightweight Windows distribution

## Current status

Foundation implementation is in place:

- Native PINE TCP client
- Explicit idle/BIOS/game/paused state model
- Serial-based PS2 cover lookup and cache
- Lazy Discord IPC publisher
- Discord external image URLs
- Windows CI/test/release build

The automatic Windows lifecycle integration is the next major implementation step.

## Setup

PCSX2 must have PINE enabled. The default Windows PINE slot is 28011.

The first run creates a config.toml next to the executable. Put the Discord application client ID under:

[discord]
client_id = "YOUR_DISCORD_APPLICATION_ID"

The client ID is not a Discord account token.

## Development

Use a current stable Rust toolchain:

cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release

## References

- PCSX2 PINE: https://github.com/PCSX2/pcsx2/blob/master/pcsx2/PINE.cpp
- PINE reference implementation: https://github.com/GovanifY/pine
- PS2 covers: https://github.com/xlenore/ps2-covers
- Extreme-InfiniTV Discord implementation: https://github.com/infinitel8p/Extreme-InfiniTV
