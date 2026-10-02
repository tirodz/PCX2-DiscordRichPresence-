use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::{blocking::Client, Url};

const COVER_BASE: &str =
    "https://raw.githubusercontent.com/xlenore/ps2-covers/main/covers/default/";
/// Discord Rich Presence renders the large image as a square. Use a smart
/// square crop so portrait PS2 covers fill the card more naturally without stretching.
const DISCORD_COVER_PROXY: &str = "https://wsrv.nl/";
const DISCORD_COVER_SIZE: &str = "1024";

/// How long a "this serial has no cover" answer is remembered before the
/// lookup is tried again.
const MISSING_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

pub struct CoverCache {
    root: PathBuf,
    http: Client,
}

impl CoverCache {
    pub fn new() -> Result<Self> {
        Self::with_root(
            ProjectDirs::from("com", "tirodz", "PCSX2DiscordRichPresence")
                .map(|d| d.cache_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from("cache")),
        )
    }

    pub fn with_root(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;

        let http = Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent("PCSX2-DiscordRichPresence")
            .build()?;

        Ok(Self { root, http })
    }

    /// Resolve the public cover URL for a game serial.
    ///
    /// Returns `Ok(None)` when the serial is unknown or no cover exists.
    /// Network and cache errors are logged by the caller and must never
    /// prevent the text presence from being published.
    pub fn cover_url(&self, serial: &str) -> Result<Option<String>> {
        self.cover_url_from(COVER_BASE, serial)
    }

    fn cover_url_from(&self, base: &str, serial: &str) -> Result<Option<String>> {
        let normalized = normalize_serial(serial);
        if normalized.is_empty() {
            return Ok(None);
        }

        let cache_file = self.root.join(format!("{normalized}.jpg"));
        if cache_file.exists() {
            return Ok(Some(discord_cover_url(&public_cover_url_from(
                base,
                &normalized,
            ))?));
        }

        let missing_file = self.root.join(format!("{normalized}.missing"));
        if missing_is_fresh(&missing_file) {
            return Ok(None);
        }

        let url = public_cover_url_from(base, &normalized);
        let response = self.http.get(&url).send()?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // Remember the miss so a game without a cover does not trigger
            // a network request on every Discord refresh.
            fs::write(&missing_file, b"not found").ok();
            return Ok(None);
        }
        if !response.status().is_success() {
            return Ok(None);
        }

        let bytes = response.bytes()?;
        if bytes.is_empty() {
            return Ok(None);
        }

        fs::write(&cache_file, &bytes)
            .with_context(|| format!("writing {}", cache_file.display()))?;

        // Discord cannot fetch a user's private file:// URL. Keep the cache
        // for offline/local use, while the presence uses the public square cover URL through the image resize proxy.
        Ok(Some(discord_cover_url(&url)?))
    }
}

fn missing_is_fresh(path: &PathBuf) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .map(|age| age < MISSING_TTL)
        .unwrap_or(false)
}

/// Normalize a PS2 serial to the `ABCD-12345` form used by the cover
/// database. Handles the variants seen in the wild: different regions
/// (SLUS/SCUS/SCES/SLES/SLPS/SLPM/PBPX/...), underscores, spaces, dots and
/// lower case.
pub fn normalize_serial(value: &str) -> String {
    let trimmed = value.trim().to_uppercase();
    if trimmed.is_empty() {
        return String::new();
    }

    // Canonical form already: four letters, dash, five digits.
    let compact: String = trimmed
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if compact.len() == 9 {
        let (letters, digits) = compact.split_at(4);
        if letters.chars().all(|c| c.is_ascii_uppercase())
            && digits.chars().all(|c| c.is_ascii_digit())
        {
            return format!("{letters}-{digits}");
        }
    }

    trimmed.replace([' ', '/'], "_")
}

fn public_cover_url_from(base: &str, serial: &str) -> String {
    format!("{base}{serial}.jpg")
}

fn discord_cover_url(source_url: &str) -> Result<String> {
    let mut url = Url::parse(DISCORD_COVER_PROXY).context("parsing cover proxy URL")?;
    url.query_pairs_mut()
        .append_pair("url", source_url)
        .append_pair("w", DISCORD_COVER_SIZE)
        .append_pair("h", DISCORD_COVER_SIZE)
        .append_pair("fit", "cover")
        .append_pair("a", "attention")
        .append_pair("output", "jpg")
        .append_pair("q", "90")
        .append_pair("maxage", "604800");
    Ok(url.to_string())
}

#[allow(dead_code)]
fn public_cover_url(serial: &str) -> String {
    public_cover_url_from(COVER_BASE, serial)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::{normalize_serial, public_cover_url, CoverCache};

    #[test]
    fn normalizes_serial() {
        assert_eq!(normalize_serial("SLUS-21274"), "SLUS-21274");
        assert_eq!(normalize_serial("slus-21274"), "SLUS-21274");
        assert_eq!(normalize_serial("SLUS_212.74"), "SLUS-21274");
        assert_eq!(normalize_serial("SLUS 21274"), "SLUS-21274");
        assert_eq!(normalize_serial(" SCES-51719 "), "SCES-51719");
        assert_eq!(normalize_serial("SCUS-97199"), "SCUS-97199");
        assert_eq!(normalize_serial("SLPS-25880"), "SLPS-25880");
        assert_eq!(normalize_serial("SLPM-66209"), "SLPM-66209");
        assert_eq!(normalize_serial("PBPX-95201"), "PBPX-95201");
        assert_eq!(normalize_serial("SLES_537.56"), "SLES-53756");
        assert_eq!(normalize_serial(""), "");
    }

    #[test]
    fn builds_serial_cover_url() {
        assert!(public_cover_url("SLUS-21274").ends_with("/SLUS-21274.jpg"));
    }

    #[test]
    fn builds_square_discord_cover_url() {
        let url = super::discord_cover_url(
            "https://raw.githubusercontent.com/xlenore/ps2-covers/main/covers/default/SLUS-20946.jpg",
        )
        .unwrap();
        assert!(url.starts_with("https://wsrv.nl/?"));
        assert!(url.contains("w=1024"));
        assert!(url.contains("h=1024"));
        assert!(url.contains("fit=cover"));
        assert!(url.contains("a=attention"));
        assert!(url.contains("output=jpg"));
        assert!(url.contains("SLUS-20946.jpg"));
    }

    /// Tiny HTTP stub that serves one cover and 404s everything else.
    fn spawn_cover_server() -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            // This test performs exactly two requests: a cache hit, then a 404.
            // The normalized serial check below is served from the local cache.
            // Exit after those requests so
            // the test process can terminate cleanly.
            for stream in listener.incoming().take(2) {
                let mut stream = match stream {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let mut buffer = [0u8; 1024];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                let (status, body): (&str, &[u8]) = if request.contains("SLUS-20946.jpg") {
                    ("200 OK", b"fake-jpeg-bytes")
                } else {
                    ("404 Not Found", b"")
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                if stream.write_all(response.as_bytes()).is_err() {
                    break;
                }
                if stream.write_all(body).is_err() {
                    break;
                }
            }
        });
        (format!("http://127.0.0.1:{port}/"), handle)
    }

    #[test]
    fn caches_hits_and_misses() {
        let (base, _server) = spawn_cover_server();
        let dir = std::env::temp_dir().join(format!("pcsx2-rp-covers-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let cache = CoverCache::with_root(dir.clone()).unwrap();

        // Known cover: fetched once, then served from the cache.
        let url = cache.cover_url_from(&base, "SLUS-20946").unwrap();
        let first_url = url.as_deref().unwrap();
        assert!(first_url.starts_with("https://wsrv.nl/?"));
        assert!(first_url.contains("SLUS-20946.jpg"));
        assert!(dir.join("SLUS-20946.jpg").exists());

        // Unknown cover: remembered as missing.
        assert_eq!(cache.cover_url_from(&base, "SCES-00000").unwrap(), None);
        assert!(dir.join("SCES-00000.missing").exists());

        // Serial variants resolve to the same cache entry.
        let again = cache.cover_url_from(&base, "slus_209.46").unwrap();
        assert!(again.is_some());

        std::fs::remove_dir_all(&dir).ok();
    }
}
