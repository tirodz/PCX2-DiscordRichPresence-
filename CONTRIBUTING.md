# Contributing

Thanks for contributing to PCSX2 Discord Rich Presence.

## Before changing code

Read the existing implementation and tests first. Preserve working behavior unless there is a concrete reason to change it.

This project is intended to remain a real, polished Windows application. Avoid demo-only code, placeholder UI, unrelated generated files, or temporary debugging committed to the repository.

## Code quality

Keep changes focused and maintainable.

Run these before submitting a change:

```text
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

Add regression coverage when changing protocol handling, state detection, configuration, Discord publishing, artwork, or lifecycle behavior.

## Windows behavior

Normal operation should be quiet and console-free.

The installer should work without administrator privileges unless a future feature genuinely requires elevation. Uninstall should cleanly stop the helper, remove startup registration, and restore configuration that the application previously changed when safe to do so.

## PCSX2 integration

PINE is an implementation detail for normal users. Do not require users to understand the protocol just to use Discord Rich Presence.

Never patch PCSX2 binaries or modify unrelated settings.

## Discord

Use the project's Discord application for public release builds.

Do not ask end users for Discord tokens or private credentials.

Keep the Rich Presence concise. Use emojis only when they improve readability.

## Artwork

Keep project artwork under `assets/`.

The supplied PS2 BIOS artwork is `assets/bios/ps2-bios.png`. Do not replace it with unrelated artwork or add extra logos to the image.

Game covers are dynamic and keyed by PS2 serial. Preserve regional serial handling and graceful fallbacks.

## Commits and documentation

Use concise, human project-focused commit messages.

Examples:

- Fix BIOS artwork delivery
- Improve cover framing
- Simplify first-run setup
- Fix Discord reconnect handling

Keep README and CHANGELOG entries synchronized with real behavior.

Avoid adding tool output, agent metadata, or unrelated implementation notes to user-facing documentation.

## Releases

Never publish a release until the installer, portable ZIP, and checksum file have been built and verified.

A release is not complete until the GitHub Release page contains the actual downloadable assets.
