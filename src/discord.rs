use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use discord_rich_presence::{
    activity::{Activity, Assets, Timestamps},
    DiscordIpc, DiscordIpcClient,
};

use crate::metadata::CoverCache;
use crate::state::{unix_now, RuntimeState};

const PCSX2_LOGO: &str =
    "https://raw.githubusercontent.com/PCSX2/pcsx2/master/bin/resources/icons/AppIconLarge.png";

/// How long a published activity is considered fresh before it is sent
/// again, and how long to wait before retrying a failed Discord connection.
const REFRESH_AFTER: Duration = Duration::from_secs(90);
const RECONNECT_BACKOFF: Duration = Duration::from_secs(15);

pub struct DiscordPublisher {
    client_id: String,
    client: Option<DiscordIpcClient>,
    last_signature: String,
    last_publish: Option<Instant>,
    session_start: Option<i64>,
    game_identity: Option<String>,
    retry_after: Option<Instant>,
}

impl DiscordPublisher {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            client: None,
            last_signature: String::new(),
            last_publish: None,
            session_start: None,
            game_identity: None,
            retry_after: None,
        }
    }

    fn ensure_connected(&mut self) -> Result<()> {
        if self.client.is_some() {
            return Ok(());
        }

        let mut client = DiscordIpcClient::new(&self.client_id);
        client.connect().context("connecting to Discord IPC")?;
        self.client = Some(client);
        Ok(())
    }

    pub fn publish(&mut self, state: &RuntimeState, covers: &CoverCache) -> Result<()> {
        let signature = state.signature();
        if signature == self.last_signature
            && self
                .last_publish
                .map(|t| t.elapsed() < REFRESH_AFTER)
                .unwrap_or(false)
        {
            return Ok(());
        }

        if matches!(state, RuntimeState::Offline) {
            return self.clear();
        }

        // A failed Discord connection is retried with backoff instead of
        // on every poll, so a closed Discord client stays quiet.
        if self
            .retry_after
            .map(|t| Instant::now() < t)
            .unwrap_or(false)
        {
            return Ok(());
        }

        let cover = match state {
            RuntimeState::Game { serial, .. } => covers.cover_url(serial).ok().flatten(),
            _ => None,
        };

        let Some(plan) = self.plan(state, cover) else {
            return Ok(());
        };

        let result = self.ensure_connected().and_then(|()| {
            self.client
                .as_mut()
                .context("Discord client missing")?
                .set_activity(plan)
                .context("setting Discord activity")
        });

        match result {
            Ok(()) => {
                self.retry_after = None;
                self.last_signature = signature;
                self.last_publish = Some(Instant::now());
                Ok(())
            }
            Err(error) => {
                // Drop the broken client so the next attempt reconnects.
                self.client = None;
                self.retry_after = Some(Instant::now() + RECONNECT_BACKOFF);
                Err(error)
            }
        }
    }

    /// Build the activity for a state, tracking the game session so the
    /// elapsed timer survives pause/resume. Pure with respect to IPC, which
    /// keeps it testable.
    fn plan(&mut self, state: &RuntimeState, cover: Option<String>) -> Option<Activity<'static>> {
        let (details, state_text, timestamp) = match state {
            RuntimeState::Offline => return None,
            RuntimeState::Idle => {
                self.reset_game_session();
                ("PCSX2".to_string(), "At the Main Menu".to_string(), None)
            }
            RuntimeState::Bios { paused } => {
                self.reset_game_session();
                (
                    "PlayStation 2".to_string(),
                    if *paused {
                        "System Menu · Paused".to_string()
                    } else {
                        "System Menu".to_string()
                    },
                    None,
                )
            }
            RuntimeState::Game {
                title,
                serial,
                crc,
                version,
                paused,
            } => {
                let identity = format!("{title}\u{1f}{serial}\u{1f}{crc}\u{1f}{version}");
                if self.game_identity.as_deref() != Some(identity.as_str()) {
                    self.game_identity = Some(identity);
                    self.session_start = Some(unix_now());
                }

                (
                    title.clone(),
                    if *paused {
                        "Paused on PCSX2".to_string()
                    } else {
                        "Playing on PCSX2".to_string()
                    },
                    self.session_start,
                )
            }
        };

        let mut activity = Activity::new().details(details.clone()).state(state_text);

        let assets = if let Some(url) = cover {
            // Game cover large, PCSX2 logo small.
            Assets::new()
                .large_image(url)
                .large_text(details)
                .small_image(PCSX2_LOGO)
                .small_text("PCSX2")
        } else {
            Assets::new().large_image(PCSX2_LOGO).large_text("PCSX2")
        };
        activity = activity.assets(assets);

        if let Some(start) = timestamp {
            activity = activity.timestamps(Timestamps::new().start(start));
        }

        Some(activity)
    }

    fn reset_game_session(&mut self) {
        self.game_identity = None;
        self.session_start = None;
    }

    pub fn clear(&mut self) -> Result<()> {
        if let Some(client) = self.client.as_mut() {
            let _ = client.clear_activity();
            let _ = client.close();
        }

        self.client = None;
        self.last_signature.clear();
        self.last_publish = None;
        self.retry_after = None;
        self.reset_game_session();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::DiscordPublisher;
    use crate::state::RuntimeState;

    fn game(paused: bool) -> RuntimeState {
        RuntimeState::Game {
            title: "Gran Turismo 4".into(),
            serial: "SCES-51719".into(),
            crc: "77e61c8a".into(),
            version: "1.00".into(),
            paused,
        }
    }

    #[test]
    fn offline_produces_no_activity() {
        let mut publisher = DiscordPublisher::new("123".into());
        assert!(publisher.plan(&RuntimeState::Offline, None).is_none());
    }

    #[test]
    fn session_timer_survives_pause_and_resume() {
        let mut publisher = DiscordPublisher::new("123".into());

        publisher.plan(&game(false), None);
        let started = publisher.session_start.expect("session started");

        publisher.plan(&game(true), None);
        assert_eq!(publisher.session_start, Some(started));

        publisher.plan(&game(false), None);
        assert_eq!(publisher.session_start, Some(started));
    }

    #[test]
    fn returning_to_the_menu_ends_the_session() {
        let mut publisher = DiscordPublisher::new("123".into());

        publisher.plan(&game(false), None);
        assert!(publisher.session_start.is_some());

        publisher.plan(&RuntimeState::Idle, None);
        assert!(publisher.session_start.is_none());
        assert!(publisher.game_identity.is_none());
    }

    #[test]
    fn a_new_game_starts_a_new_session() {
        let mut publisher = DiscordPublisher::new("123".into());

        publisher.plan(&game(false), None);
        let first = publisher.session_start.unwrap();

        // Make sure the next timestamp can differ.
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let other = RuntimeState::Game {
            title: "Kingdom Hearts".into(),
            serial: "SLUS-20370".into(),
            crc: "c90c8f3d".into(),
            version: "1.00".into(),
            paused: false,
        };
        publisher.plan(&other, None);
        let second = publisher.session_start.unwrap();

        assert!(second >= first);
        assert_ne!(publisher.game_identity, None);
    }

    #[test]
    fn bios_state_has_no_timer() {
        let mut publisher = DiscordPublisher::new("123".into());
        publisher.plan(&RuntimeState::Bios { paused: false }, None);
        assert!(publisher.session_start.is_none());
    }
}
