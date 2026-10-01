use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeState {
    Offline,
    Idle,
    Bios {
        paused: bool,
    },
    Game {
        title: String,
        serial: String,
        crc: String,
        version: String,
        paused: bool,
    },
}

impl RuntimeState {
    pub fn signature(&self) -> String {
        format!("{self:?}")
    }
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64
}
