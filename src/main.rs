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
    let config = Config::load_or_create()?;

    if config.discord.client_id.trim().is_empty() {
        eprintln!("Discord client ID is not configured.");
        eprintln!("Set [discord] client_id in config.toml and run again.");
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
