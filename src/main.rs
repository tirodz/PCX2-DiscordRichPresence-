#![cfg_attr(windows, windows_subsystem = "windows")]

mod autostart;
mod config;
mod discord;
mod helper;
mod icon;
mod logging;
mod metadata;
mod pcsx2;
mod pine;
mod setup;
mod state;

use anyhow::Result;

use config::Config;

fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("--install") => {
            autostart::install()?;
            autostart::clear_stop_request()?;
        }
        Some("--uninstall") => {
            if let Ok(config) = Config::load() {
                if let Err(error) = pcsx2::restore_builtin_discord() {
                    logging::error(&format!("could not restore PCSX2 Discord presence setting: {error:#}"));
                } else {
                    let _ = config;
                }
            }
            autostart::uninstall()?;
        }
        Some("--status") => {
            println!(
                "Automatic startup: {}",
                if autostart::installed()? {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            match helper::running_pid() {
                Some(pid) => println!("Background helper: running (pid {pid})"),
                None => println!("Background helper: not running"),
            }
        }
        Some("--background") => return helper::run(),
        Some("--setup") => return setup::run_wizard(),
        Some("--settings") => return setup::run_settings(),
        Some("--help") | Some("-h") => {
            println!("PCSX2 Discord Rich Presence");
            println!();
            println!("  (no arguments)  Open the setup wizard or settings");
            println!("  --setup         Run the first-time setup wizard");
            println!("  --settings      Open the settings window");
            println!("  --background    Run the background presence helper");
            println!("  --install       Start automatically when Windows logs in");
            println!("  --uninstall     Remove automatic startup and stop the helper");
            println!("  --status        Show startup and helper status");
        }
        Some(argument) => {
            anyhow::bail!("unknown argument: {argument} (try --help)");
        }
        None => {
            // Double-click: set up on first run, adjust settings afterwards.
            let configured = Config::load().map(|c| c.is_configured()).unwrap_or(false);
            if configured {
                return setup::run_settings();
            }
            return setup::run_wizard();
        }
    }

    Ok(())
}
