use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::blocking::Client;

const COVER_BASE: &str =
    "https://raw.githubusercontent.com/xlenore/ps2-covers/main/covers/default/";

pub struct CoverCache {
    root: PathBuf,
    http: Client,
}

impl CoverCache {
    pub fn new() -> Result<Self> {
        let root = ProjectDirs::from("com", "tirodz", "PCSX2DiscordRichPresence")
            .map(|d| d.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("cache"));
        fs::create_dir_all(&root)?;

        let http = Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent("PCSX2-DiscordRichPresence")
            .build()?;

        Ok(Self { root, http })
    }

    pub fn cover_url(&self, serial: &str) -> Result<Option<String>> {
        let normalized = normalize_serial(serial);
        if normalized.is_empty() {
            return Ok(None);
        }

        let cache_file = self.root.join(format!("{normalized}.jpg"));
        if cache_file.exists() {
            return Ok(Some(public_cover_url(&normalized)));
        }

        let url = format!("{COVER_BASE}{normalized}.jpg");
        let response = self.http.get(&url).send()?;
        if !response.status().is_success() {
            return Ok(None);
        }

        let bytes = response.bytes()?;
        if bytes.is_empty() {
            return Ok(None);
        }

        fs::write(&cache_file, &bytes)
            .with_context(|| format!("writing {}", cache_file.display()))?;

        // Discord cannot fetch a user's private file:// URL. Keep the cache for
        // offline/local metadata work, while Presence uses the canonical public
        // cover URL through Discord's media proxy.
        Ok(Some(public_cover_url(&normalized)))
    }
}

fn normalize_serial(value: &str) -> String {
    value.trim().replace(' ', "_").replace('/', "_")
}

fn public_cover_url(serial: &str) -> String {
    format!("{COVER_BASE}{serial}.jpg")
}

#[allow(dead_code)]
fn _cache_path(root: &Path, serial: &str) -> PathBuf {
    root.join(format!("{serial}.jpg"))
}

#[cfg(test)]
mod tests {
    use super::{normalize_serial, public_cover_url};

    #[test]
    fn normalizes_serial() {
        assert_eq!(normalize_serial("SLUS-21274"), "SLUS-21274");
        assert_eq!(normalize_serial("SLUS 21274"), "SLUS_21274");
    }

    #[test]
    fn builds_serial_cover_url() {
        assert!(public_cover_url("SLUS-21274").ends_with("/SLUS-21274.jpg"));
    }
}
