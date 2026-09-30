use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub pine: PineConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default = "default_poll")]
    pub poll_seconds: u64,
    #[serde(default = "default_retry")]
    pub retry_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PineConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DiscordConfig {
    #[serde(default)]
    pub client_id: String,
}

fn default_host() -> String { "127.0.0.1".into() }
fn default_port() -> u16 { 28011 }
fn default_poll() -> u64 { 2 }
fn default_retry() -> u64 { 3 }

impl Default for Config {
    fn default() -> Self {
        Self {
            pine: PineConfig::default(),
            discord: DiscordConfig::default(),
            poll_seconds: default_poll(),
            retry_seconds: default_retry(),
        }
    }
}

impl Default for PineConfig {
    fn default() -> Self {
        Self { host: default_host(), port: default_port() }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| BaseDirs::new().map(|d| d.data_local_dir().to_path_buf()).unwrap_or_default())
            .join("config.toml")
    }

    pub fn load_or_create() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            let template = "[pine]\nhost = \"127.0.0.1\"\nport = 28011\n\n[discord]\nclient_id = \"\"\n\npoll_seconds = 2\nretry_seconds = 3\n";
            fs::write(&path, template).with_context(|| format!("writing {}", path.display()))?;
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}
