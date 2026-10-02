use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub pcsx2: Pcsx2Config,
    #[serde(default)]
    pub pine: PineConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default = "default_poll")]
    pub poll_seconds: u64,
    #[serde(default = "default_retry")]
    pub retry_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Pcsx2Config {
    /// Full path to the PCSX2 executable chosen during setup, for example
    /// `C:\Emulators\PCSX2\pcsx2-qt.exe`.
    #[serde(default)]
    pub exe_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PineConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscordConfig {
    #[serde(default)]
    pub client_id: String,
}

fn default_host() -> String {
    "127.0.0.1".into()
}

fn default_port() -> u16 {
    28011
}

fn default_poll() -> u64 {
    2
}

fn default_retry() -> u64 {
    3
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pcsx2: Pcsx2Config::default(),
            pine: PineConfig::default(),
            discord: DiscordConfig::default(),
            poll_seconds: default_poll(),
            retry_seconds: default_retry(),
        }
    }
}

impl Default for PineConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| {
                BaseDirs::new()
                    .map(|d| d.data_local_dir().to_path_buf())
                    .unwrap_or_default()
            })
            .join("config.toml")
    }

    pub fn load() -> Result<Self> {
        Self::load_from(&Self::path())
    }

    pub fn load_or_create() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        Self::load()
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).context("serializing configuration")?;
        fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Setup is considered complete once both the PCSX2 executable and the
    /// Discord application ID are known.
    pub fn is_configured(&self) -> bool {
        !self.pcsx2.exe_path.trim().is_empty()
            && (!self.discord.client_id.trim().is_empty()
                || !option_env!("PCSX2_DISCORD_CLIENT_ID")
                    .unwrap_or("")
                    .trim()
                    .is_empty())
    }
}

/// Find a likely PCSX2 executable in common Windows installation locations.
///
/// This is intentionally conservative. The setup wizard falls back to the
/// Browse dialog instead of performing an expensive full-disk search.
pub fn detect_pcsx2_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        let root = PathBuf::from(program_files).join("PCSX2");
        candidates.push(root.join("pcsx2-qt.exe"));
        candidates.push(root.join("pcsx2.exe"));
    }

    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        let root = PathBuf::from(program_files_x86).join("PCSX2");
        candidates.push(root.join("pcsx2-qt.exe"));
        candidates.push(root.join("pcsx2.exe"));
    }

    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        let root = PathBuf::from(local_app_data).join("Programs").join("PCSX2");
        candidates.push(root.join("pcsx2-qt.exe"));
        candidates.push(root.join("pcsx2.exe"));
    }

    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let profile = PathBuf::from(user_profile);
        for channel in ["pcsx2", "pcsx2-dev"] {
            let root = profile
                .join("scoop")
                .join("apps")
                .join(channel)
                .join("current");
            candidates.push(root.join("pcsx2-qt.exe"));
            candidates.push(root.join("pcsx2.exe"));
        }
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path_var) {
            candidates.push(directory.join("pcsx2-qt.exe"));
            candidates.push(directory.join("pcsx2.exe"));
        }
    }

    candidates.into_iter().find(|path| path.is_file())
}

/// Check that a selected file plausibly is the PCSX2 executable.
///
/// The file must exist and its name must look like a PCSX2 build, which
/// covers the regular installer (`pcsx2.exe`), the Qt builds
/// (`pcsx2-qt.exe`) and portable/nightly names such as
/// `pcsx2-v2.3.100-windows-x64-Qt.exe`.
pub fn validate_pcsx2_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("Choose the PCSX2 executable first.".to_string());
    }
    if !path.exists() {
        return Err(format!("{} does not exist.", path.display()));
    }
    if !path.is_file() {
        return Err(format!("{} is not a file.", path.display()));
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let is_exe = name.ends_with(".exe") || cfg!(not(windows));
    let looks_like_pcsx2 = name.starts_with("pcsx2");

    if !is_exe {
        return Err(format!(
            "{name} is not a Windows executable. Select the PCSX2 .exe file, for example pcsx2-qt.exe."
        ));
    }
    if !looks_like_pcsx2 {
        return Err(format!(
            "{name} does not look like a PCSX2 executable. PCSX2 builds are named pcsx2.exe, pcsx2-qt.exe or similar."
        ));
    }
    Ok(())
}

/// Discord application IDs are numeric snowflakes.
pub fn validate_client_id(value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Enter your Discord application ID.".to_string());
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err(
            "The application ID only contains digits. Copy it from the Discord Developer Portal, General Information page."
                .to_string(),
        );
    }
    if !(15..=25).contains(&trimmed.len()) {
        return Err(format!(
            "Discord application IDs are usually 17-20 digits long; this one has {}. Double-check what you copied.",
            trimmed.len()
        ));
    }
    Ok(())
}

pub fn validate_pine(host: &str, port: &str) -> Result<u16, String> {
    if host.trim().is_empty() {
        return Err("The PINE host cannot be empty. Use 127.0.0.1 for a local PCSX2.".to_string());
    }
    let port: u16 = port
        .trim()
        .parse()
        .map_err(|_| "The PINE slot must be a number between 1 and 65535.".to_string())?;
    if port == 0 {
        return Err("The PINE slot must be between 1 and 65535.".to_string());
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_toml() {
        let mut config = Config::default();
        config.pcsx2.exe_path = "C:\\Emulators\\PCSX2\\pcsx2-qt.exe".into();
        config.discord.client_id = "123456789012345678".into();
        config.pine.port = 28012;

        let text = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();

        assert_eq!(parsed.pcsx2.exe_path, config.pcsx2.exe_path);
        assert_eq!(parsed.discord.client_id, config.discord.client_id);
        assert_eq!(parsed.pine.port, 28012);
        assert_eq!(parsed.pine.host, "127.0.0.1");
        assert!(parsed.is_configured());
    }

    #[test]
    fn defaults_apply_to_partial_files() {
        let parsed: Config =
            toml::from_str("[discord]\nclient_id = \"123456789012345678\"\n").unwrap();
        assert_eq!(parsed.pine.host, "127.0.0.1");
        assert_eq!(parsed.pine.port, 28011);
        assert_eq!(parsed.poll_seconds, 2);
        assert_eq!(parsed.retry_seconds, 3);
        assert!(!parsed.is_configured());
    }

    #[test]
    fn save_and_load_from_disk() {
        let dir = std::env::temp_dir().join(format!("pcsx2-rp-config-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        let mut config = Config::default();
        config.pcsx2.exe_path = "D:\\Games\\PCSX2\\pcsx2.exe".into();
        config.save_to(&path).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.pcsx2.exe_path, config.pcsx2.exe_path);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn client_id_validation() {
        assert!(validate_client_id("123456789012345678").is_ok());
        assert!(validate_client_id("").is_err());
        assert!(validate_client_id("abc123").is_err());
        assert!(validate_client_id("12345").is_err());
        assert!(validate_client_id(" 123456789012345678 ").is_ok());
    }

    #[test]
    fn pine_validation() {
        assert_eq!(validate_pine("127.0.0.1", "28011").unwrap(), 28011);
        assert!(validate_pine("", "28011").is_err());
        assert!(validate_pine("127.0.0.1", "abc").is_err());
        assert!(validate_pine("127.0.0.1", "0").is_err());
        assert!(validate_pine("127.0.0.1", "70000").is_err());
    }

    #[test]
    fn pcsx2_path_validation_names() {
        // Name checks run before existence for non-existent paths, so use a
        // real file with varying names.
        let dir = std::env::temp_dir().join(format!("pcsx2-rp-validate-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let good = dir.join("pcsx2-qt.exe");
        fs::write(&good, b"dummy").unwrap();
        assert!(validate_pcsx2_path(&good).is_ok());

        let nightly = dir.join("pcsx2-v2.3.100-windows-x64-Qt.exe");
        fs::write(&nightly, b"dummy").unwrap();
        assert!(validate_pcsx2_path(&nightly).is_ok());

        let wrong = dir.join("notepad.exe");
        fs::write(&wrong, b"dummy").unwrap();
        assert!(validate_pcsx2_path(&wrong).is_err());

        let missing = dir.join("pcsx2.exe");
        assert!(validate_pcsx2_path(&missing).is_err());

        fs::remove_dir_all(&dir).ok();
    }
}
