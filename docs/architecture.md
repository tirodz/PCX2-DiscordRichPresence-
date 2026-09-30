# Architecture

## PCSX2

The integration uses PCSX2's native PINE IPC rather than window-title scraping, injection, memory scanning, or modified emulator binaries.

On Windows, the current PCSX2 source uses TCP on localhost and defines 28011 as the default PINE slot. PINE provides title, serial/ID, CRC/UUID, game version, and emulator status.

## Runtime states

- Offline: PINE is unreachable, so PCSX2 is not available to the integration.
- Idle: PINE is reachable but the VM reports Shutdown. This represents PCSX2 being open without an active PS2 VM.
- BIOS: a VM is active but title/serial metadata is unavailable.
- Game: title/serial metadata is available.
- Paused: represented on BIOS/Game states.

The BIOS classification is intentionally conservative. Real PCSX2 testing must confirm exactly what the current BIOS/system-menu VM reports.

## Discord

The Discord connection is lazy: the application does not open Discord IPC until it has a PCSX2 state to publish.

The last activity signature is cached to avoid unnecessary SET_ACTIVITY traffic. A periodic refresh prevents stale activity, while Discord disconnects are recovered on the next publish.

Extreme-InfiniTV was used as a reference for this lifecycle pattern: lazy connection, mutex-protected Discord client, quiet handling when Discord is absent, explicit idle presence, and clear/disconnect behavior.

## Covers

Game covers are resolved by serial through the xlenore PS2 cover repository. The cover is cached locally, but the Discord activity uses the public canonical cover URL because Discord's media proxy cannot retrieve a user's private local file.

A missing cover never prevents the text presence from being published.

## Automatic lifecycle

Windows setup is handled with a per-user startup entry. The helper starts silently at logon, waits for PCSX2's PINE endpoint, and does not connect to Discord until there is emulator state to publish.

When PCSX2 closes, PINE becomes unreachable and the helper clears the Discord activity. It remains resident in the background so a later PCSX2 launch is picked up without another manual launch.

The startup entry can be removed with `--uninstall`. No administrator privileges or Windows service are required.
