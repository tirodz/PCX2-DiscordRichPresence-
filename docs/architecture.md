# Architecture

## PCSX2

The integration uses PCSX2's native PINE IPC rather than window-title
scraping, injection, memory scanning, or modified emulator binaries.

On Windows, PINE speaks TCP on localhost, slot 28011 by default (the slot is
configurable in PCSX2 and in this app's settings). PINE provides the game
title, serial/ID, CRC/UUID, game version, and the emulator status
(Running / Paused / Shutdown).

Replies are framed as a little-endian u32 total length (including the four
length bytes), one result byte (`0` = ok, `0xFF` = fail), then the payload.
String payloads are a u32 length (including the NUL terminator) followed by
the string bytes. When no VM is running, the metadata opcodes answer with
fail, which the client treats as "no game metadata".

## Runtime states

- Offline: PINE is unreachable, so PCSX2 is not available to the integration.
- Idle: PINE is reachable but the VM reports Shutdown. This represents PCSX2
  being open without an active PS2 VM.
- BIOS: a VM is active but title/serial metadata is unavailable.
- Game: title/serial metadata is available.
- Paused: represented on BIOS/Game states.

The BIOS classification is intentionally conservative. Real PCSX2 testing
must confirm exactly what the current BIOS/system-menu VM reports.

## Helper loop

The background helper keeps one PINE connection open across polls instead of
reconnecting every cycle. Connection and query failures are debounced: only
after several consecutive failures is PCSX2 considered gone and the Discord
activity cleared. Short hiccups while a game boots therefore neither clear
the activity nor reset the session timer.

While PINE is unreachable the loop just sleeps between connection attempts,
so an idle helper costs essentially nothing.

## Discord

The Discord connection is lazy: the application does not open Discord IPC
until it has a PCSX2 state to publish.

The last activity signature is cached to avoid unnecessary SET_ACTIVITY
traffic, with a periodic refresh so the activity never goes stale. A failed
publish drops the IPC client and retries with backoff, so a closed Discord
client produces one log line instead of a stream of errors.

The game session (title + serial + CRC + version) owns the elapsed timer.
Pause/resume republishes the state text but keeps the original start
timestamp; leaving to the menu or closing PCSX2 ends the session.

## Covers

Game covers are resolved by serial through the xlenore PS2 cover repository.
Serials are normalized to the `ABCD-12345` form first. The source cover is
cached locally, while Discord receives a public 512x512 rendition through
wsrv.nl with aspect-ratio-preserving contain/letterboxing so portrait box art
does not get cropped by Discord's square large-image presentation. Serials
without a cover get a `.missing` marker (valid for a week) so they are not
re-requested on every refresh.

A missing cover never prevents the text presence from being published; the
PCSX2 logo is the fallback artwork.

## Configuration and setup

Configuration lives in `config.toml` next to the executable: the PCSX2
executable path, the PINE host/slot, the Discord application ID, and the poll
intervals. The first-run wizard collects these interactively (validating the
PCSX2 path and the application ID, and optionally testing the PINE
connection), writes the config, registers startup, and launches the helper.
The settings window edits the same fields later.

## Built-in Discord presence

PCSX2 has its own optional Discord presence controlled by
`EmuCore/EnableDiscordPresence`. During setup this app disables that setting
when a PCSX2 configuration file is available, records the previous value, and
restores it on uninstall. If the emulator has never created a settings file,
PCSX2's default is already off, so no file is created just for the takeover.

## Automatic lifecycle

Windows startup is a per-user `Run` registry entry pointing at the
executable with `--background`. No administrator privileges or Windows
service are required.

The helper starts silently at logon, refuses to run twice (PID file plus a
liveness check), and waits for PCSX2's PINE endpoint. When PCSX2 closes, the
activity is cleared; the helper stays resident so the next launch is picked
up automatically.

`--uninstall` removes the registry entry and asks the running helper to exit
through a `.stop` file next to the executable, which the installer uses for
clean uninstalls. The setup wizard and settings window start and restart the
helper through the same mechanism.
