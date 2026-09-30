use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use discord_rich_presence::{
    activity::{Activity, Assets, Timestamps},
    DiscordIpc, DiscordIpcClient,
};

use crate::metadata::CoverCache;
use crate::state::{unix_now, RuntimeState};

const PCSX2_LOGO =
    "https://raw.githubusercontent.com/PCSX2/pcsx2/master/pcsx2/Icons/pcsx2.svg";

pub struct DiscordPublisher {
    client_id: String,
    client: Option<DiscordIpcClient>,
    last_signature: String,
    last_publish: Option<Instant>,
    session_start: Option<i64>,
}

impl DiscordPublisher {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            client: None,
            last_signature: String::new(),
            last_publish: None,
            session_start: None,
        }
    }

    fn ensure_connected(&mut self) -> Result<()> {
        if self.client.is_some() {
            return Ok(());
        }

        let mut client =
            DiscordIpcClient::new(&self.client_id).context("creating Discord IPC client")?;
        client.connect().context("connecting to Discord IPC")?;
        self.client = Some(client);
        Ok(())
    }

    pub fn publish(&mut self, state: &RuntimeState, covers: &CoverCache) -> Result<()> {
        let signature = state.signature();
        if signature == self.last_signature
            && self
                .last_publish
                .map(|t| t.elapsed() < Duration::from_secs(90))
                .unwrap_or(false)
        {
            return Ok(());
        }

        self.ensure_connected()?;

        let (details, state_text, cover, timestamp) = match state {
            RuntimeState::Idle => (
                "PCSX2".to_string(),
                "At the Main Menu".to_string(),
                None,
                None,
            ),
            RuntimeState::Bios { paused } => (
                "PlayStation 2".to_string(),
                if *paused {
                    "System Menu · Paused".to_string()
                } else {
                    "System Menu".to_string()
                },
                None,
                None,
            ),
            RuntimeState::Game {
                title,
                serial,
                paused,
                ..
            } => {
                let cover = covers.cover_url(serial).ok().flatten();
                if self.session_start.is_none() || signature != self.last_signature {
                    self.session_start = Some(unix_now());
                }
                (
                    title.clone(),
                    if *paused {
                        "Paused on PCSX2".to_string()
                    } else {
                        "Playing on PCSX2".to_string()
                    },
                    cover,
                    self.session_start,
                )
            }
            RuntimeState::Offline => return self.clear(),
        };

        let mut activity = Activity::new().details(&details).state(&state_text);
        let mut assets = Assets::new().small_image(PCSX2_LOGO).small_text("PCSX2");

        if let Some(url) = cover {
            assets = assets.large_image(&url).large_text(&details);
        } else {
            assets = assets.large_image(PCSX2_LOGO).large_text("PCSX2");
        }

        activity = activity.assets(assets);

        if let Some(start) = timestamp {
            activity = activity.timestamps(Timestamps::new().start(start));
        }

        self.client
            .as_mut()
            .context("Discord client missing")?
            .set_activity(activity)
            .context("setting Discord activity")?;

        self.last_signature = signature;
        self.last_publish = Some(Instant::now());
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        if let Some(client) = self.client.as_mut() {
            let _ = client.clear_activity();
            let _ = client.close();
        }
        self.client = None;
        self.last_signature.clear();
        self.last_publish = None;
        self.session_start = None;
        Ok(())
    }
}
