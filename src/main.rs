#![cfg_attr(windows, windows_subsystem = "windows")]

mod autostart;
mod config;
mod discord;
mod metadata;
mod pine;
mod state;

use std::thread;
use std::time::Duration;

use anyhow::Result;

use config::Config;
use discord::DiscordPublisher;
use metadata::CoverCache;
use pine::PineClient;
use state::RuntimeState;

fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("--install") => {
            autostart::install()?;
            return Ok(());
        }
        Some("--uninstall") => {
            autostart::uninstall()?;
            return Ok(());
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
            return Ok(());
        }
        Some("--background") | None => {}
        Some("--help") | Some("-h") => {
            println!("PCSX2 Discord Rich Presence");
            println!();
            println!("  --install    Start automatically when Windows logs in");
            println!("  --uninstall  Remove automatic startup");
            println!("  --status     Show automatic startup status");
            return Ok(());
        }
        Some(argument) => {
            anyhow::bail!("unknown argument: {argument}");
        }
    }

    run()
}

fn run() -> Result<()> {
    let config = Config::load_or_create()?;

    if config.discord.client_id.trim().is_empty() {
        return Ok(());
    }

    let covers = CoverCache::new()?;
    let mut discord = DiscordPublisher::new(config.discord.client_id.clone());
    let mut previous = RuntimeState::Offline;

    loop {
        match PineClient::connect(&config.pine.host, config.pine.port) {
            Ok(mut pine) => match pine.read_state() {
                Ok(current) => {
                    if current != previous {
                        if let Err(error) = discord.publish(&current, &covers) {
                            eprintln!("Discord presence update failed: {error:#}");
                        }
                        previous = current;
                    } else if let Err(error) = discord.publish(&current, &covers) {
                        eprintln!("Discord presence refresh failed: {error:#}");
                    }
                }
                Err(error) => {
                    eprintln!("PINE query failed: {error}");
                    if previous != RuntimeState::Offline {
                        let _ = discord.clear();
                        previous = RuntimeState::Offline;
                    }
                }
            },
            Err(_) => {
                if previous != RuntimeState::Offline {
                    let _ = discord.clear();
                    previous = RuntimeState::Offline;
                }
                thread::sleep(Duration::from_secs(config.retry_seconds.max(1)));
                continue;
            }
        }

        thread::sleep(Duration::from_secs(config.poll_seconds.max(1)));
    }
}
