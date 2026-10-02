//! The background helper loop: watch PINE, publish Discord activity.

use std::process::Stdio;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::autostart;
use crate::config::Config;
use crate::discord::DiscordPublisher;
use crate::logging;
use crate::metadata::CoverCache;
use crate::pine::PineClient;
use crate::state::RuntimeState;

/// How many consecutive PINE failures are tolerated before PCSX2 is
/// considered gone. Short hiccups while a game boots must not clear the
/// Discord activity or reset the session timer.
const MAX_FAILURES: u32 = 3;

pub fn run() -> Result<()> {
    logging::init();
    logging::info("helper starting");

    autostart::clear_stop_request()?;

    let config = Config::load_or_create()?;
    let client_id = if config.discord.client_id.trim().is_empty() {
        option_env!("PCSX2_DISCORD_CLIENT_ID").unwrap_or("").to_string()
    } else {
        config.discord.client_id.clone()
    };
    if client_id.trim().is_empty() {
        logging::info("no Discord application configured yet; run the setup wizard first");
        return Ok(());
    }

    // Single-instance guard. A stale PID file from a crashed helper is fine:
    // the liveness check fails and the file is overwritten below.
    if let Some(pid) = running_pid() {
        logging::info(&format!(
            "another helper instance is already running (pid {pid})"
        ));
        return Ok(());
    }
    autostart::write_pid()?;

    let covers = CoverCache::new()?;
    let mut discord = DiscordPublisher::new(client_id);
    let mut pine: Option<PineClient> = None;
    let mut failures = 0u32;
    let mut had_presence = false;

    loop {
        if autostart::stop_requested()? {
            logging::info("stop requested; shutting down");
            let _ = discord.clear();
            let _ = autostart::clear_pid();
            return Ok(());
        }

        if pine.is_none() {
            match PineClient::connect(&config.pine.host, config.pine.port) {
                Ok(client) => pine = Some(client),
                Err(_) => {
                    failures = failures.saturating_add(1);
                    if failures >= MAX_FAILURES && had_presence {
                        logging::info("PCSX2 is gone; clearing the Discord activity");
                        let _ = discord.publish(&RuntimeState::Offline, &covers);
                        had_presence = false;
                    }
                    thread::sleep(Duration::from_secs(config.retry_seconds.max(1)));
                    continue;
                }
            }
        }

        let client = pine.as_mut().expect("pine client just connected");
        match client.read_state() {
            Ok(state) => {
                failures = 0;
                match discord.publish(&state, &covers) {
                    Ok(()) => had_presence = true,
                    Err(error) => logging::error(&format!("Discord update failed: {error:#}")),
                }
            }
            Err(error) => {
                logging::error(&format!("PINE query failed ({error}); reconnecting"));
                pine = None;
                failures = failures.saturating_add(1);
                if failures >= MAX_FAILURES && had_presence {
                    logging::info("lost the PCSX2 connection; clearing the Discord activity");
                    let _ = discord.publish(&RuntimeState::Offline, &covers);
                    had_presence = false;
                }
            }
        }

        thread::sleep(Duration::from_secs(config.poll_seconds.max(1)));
    }
}

/// Start the background helper as a detached, windowless process.
pub fn start_detached() -> Result<()> {
    autostart::clear_stop_request()?;
    let executable = std::env::current_exe()?;

    let mut command = std::process::Command::new(executable);
    command
        .arg("--background")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        command.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    command.spawn().context("starting the background helper")?;
    Ok(())
}

/// Ask a running helper to exit. The helper notices within one poll cycle.
pub fn request_stop() -> Result<()> {
    autostart::request_stop()
}

/// PID of the currently running helper, if any.
pub fn running_pid() -> Option<u32> {
    autostart::read_pid().filter(|&pid| autostart::pid_alive(pid))
}

/// Wait until no helper is running anymore. Returns false on timeout.
pub fn wait_until_stopped(timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if running_pid().is_none() {
            return true;
        }
        thread::sleep(Duration::from_millis(200));
    }
    running_pid().is_none()
}
