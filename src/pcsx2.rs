//! Safe management of PCSX2's built-in Discord presence setting.
//!
//! PCSX2 stores the global switch as EmuCore/EnableDiscordPresence in
//! PCSX2.ini. This module turns it off for the custom publisher and keeps
//! enough state to restore the user's original setting when the app is
//! uninstalled.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const SECTION: &str = "EmuCore";
const KEY: &str = "EnableDiscordPresence";
const BACKUP_FILE: &str = ".pcsx2-discord-rpc-backup.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Backup {
    config_path: String,
    /// Exact value as it appeared before we took ownership. None means the
    /// key was not explicitly present and PCSX2 was using its default.
    original_value: Option<String>,
}

pub fn disable_builtin_discord(executable: &Path) -> Result<()> {
    let Some(config_path) = locate_config(executable) else {
        // PCSX2 defaults this setting to false. There is nothing to change
        // until a settings file exists.
        return Ok(());
    };

    let backup_path = backup_path()?;
    let existing_backup = load_backup(&backup_path)?;

    // If the selected PCSX2 installation changed, restore the old one before
    // taking ownership of the new configuration file.
    if let Some(backup) = existing_backup.as_ref() {
        if Path::new(&backup.config_path) != config_path {
            restore_backup(backup)?;
            remove_backup(&backup_path)?;
        }
    }

    let current_text = fs::read_to_string(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let current_raw = find_setting(&current_text, SECTION, KEY);
    let current_enabled = current_raw
        .as_deref()
        .map(parse_bool)
        .transpose()
        .map_err(|error| anyhow::anyhow!("parsing {KEY} in {}: {error}", config_path.display()))?
        .unwrap_or(false);

    let same_backup = existing_backup
        .as_ref()
        .filter(|backup| Path::new(&backup.config_path) == config_path);

    if current_enabled {
        let new_text = set_setting(&current_text, SECTION, KEY, "false");
        if new_text != current_text {
            fs::write(&config_path, new_text)
                .with_context(|| format!("writing {}", config_path.display()))?;
        }
    }

    if same_backup.is_none() {
        let backup = Backup {
            config_path: config_path.to_string_lossy().into_owned(),
            original_value: current_raw,
        };
        fs::write(&backup_path, toml::to_string_pretty(&backup)?)
            .with_context(|| format!("writing {}", backup_path.display()))?;
    }

    Ok(())
}

pub fn restore_builtin_discord() -> Result<()> {
    let backup_path = backup_path()?;
    let Some(backup) = load_backup(&backup_path)? else {
        return Ok(());
    };

    restore_backup(&backup)?;
    remove_backup(&backup_path)?;
    Ok(())
}

fn restore_backup(backup: &Backup) -> Result<()> {
    let path = PathBuf::from(&backup.config_path);
    if !path.exists() {
        return Ok(());
    }

    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let current_raw = find_setting(&text, SECTION, KEY);

    // Only undo our change if the effective setting is still disabled. If
    // somebody changed it manually after setup, leave their newer choice
    // alone instead of overwriting it during uninstall.
    let Some(current_raw) = current_raw else {
        // The setting was removed while the app was installed. Respect that
        // manual change rather than recreating it during uninstall.
        return Ok(());
    };
    let current_enabled = parse_bool(&current_raw)
        .map_err(|error| anyhow::anyhow!("parsing {KEY} in {}: {error}", path.display()))?;

    if !current_enabled {
        let new_text = match backup.original_value.as_deref() {
            Some(value) => set_setting(&text, SECTION, KEY, value),
            None => remove_setting(&text, SECTION, KEY),
        };
        if new_text != text {
            fs::write(&path, new_text).with_context(|| format!("writing {}", path.display()))?;
        }
    }

    Ok(())
}

fn locate_config(executable: &Path) -> Option<PathBuf> {
    let app_root = executable.parent()?;
    let mut candidates = Vec::new();

    // Portable PCSX2 keeps its data beside the emulator (or in the path
    // written inside portable.txt).
    let portable_txt = app_root.join("portable.txt");
    let portable_ini = app_root.join("portable.ini");

    if portable_txt.exists() || portable_ini.exists() {
        let data_root = match fs::read_to_string(&portable_txt) {
            Ok(value) if !value.trim().is_empty() => {
                let value = PathBuf::from(value.trim());
                if value.is_absolute() {
                    value
                } else {
                    app_root.join(value)
                }
            }
            _ => app_root.to_path_buf(),
        };
        candidates.push(data_root.join("inis").join("PCSX2.ini"));
    }

    // A custom data path can be selected in PCSX2. We cannot query that path
    // without launching/modifying the emulator, so prefer an existing
    // in-directory config first and then the standard Windows data location.
    candidates.push(app_root.join("inis").join("PCSX2.ini"));

    if let Some(profile) = std::env::var_os("USERPROFILE") {
        let profile = PathBuf::from(profile);
        candidates.push(
            profile
                .join("Documents")
                .join("PCSX2")
                .join("inis")
                .join("PCSX2.ini"),
        );
        candidates.push(
            profile
                .join("OneDrive")
                .join("Documents")
                .join("PCSX2")
                .join("inis")
                .join("PCSX2.ini"),
        );
    }

    candidates.into_iter().find(|path| path.is_file())
}

fn backup_path() -> Result<PathBuf> {
    std::env::current_exe()?
        .parent()
        .map(|path| path.join(BACKUP_FILE))
        .ok_or_else(|| anyhow::anyhow!("executable directory is unavailable"))
}

fn load_backup(path: &Path) -> Result<Option<Backup>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(Some(
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?,
    ))
}

fn remove_backup(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn parse_bool(value: &str) -> Result<bool, &'static str> {
    match value.trim().trim_matches('"').to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err("expected a boolean value"),
    }
}

fn find_setting(text: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;

    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_section = line[1..line.len() - 1].trim() == section;
            continue;
        }

        if !in_section || line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }

        let value = value.split([';', '#']).next().unwrap_or(value).trim();

        return Some(value.to_string());
    }

    None
}

fn set_setting(text: &str, section: &str, key: &str, value: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();

    let mut in_section = false;
    let mut section_found = false;

    for line in lines.iter_mut() {
        let replacement = {
            let (body, eol) = split_eol(line);
            let trimmed = body.trim();

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_section = trimmed[1..trimmed.len() - 1].trim() == section;
                if in_section {
                    section_found = true;
                }
                None
            } else if in_section {
                if let Some((name, _)) = body.split_once('=') {
                    if name.trim() == key {
                        let eq = body.find('=').expect("split_once found =");
                        let after = &body[eq + 1..];
                        let comment_offset = match (after.find(';'), after.find('#')) {
                            (Some(a), Some(b)) => Some(a.min(b)),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None,
                        };
                        let suffix = if let Some(offset) = comment_offset {
                            let before_comment = &after[..offset];
                            let whitespace_start = before_comment
                                .char_indices()
                                .rev()
                                .take_while(|(_, ch)| ch.is_whitespace())
                                .last()
                                .map(|(index, _)| index)
                                .unwrap_or(before_comment.len());
                            &after[whitespace_start..]
                        } else {
                            ""
                        };
                        Some(format!("{} {}{}{}", &body[..=eq], value, suffix, eol))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(replacement) = replacement {
            *line = replacement;
            return lines.concat();
        }
    }

    let setting = format!("{} = {}{}", key, value, newline);

    if section_found {
        let mut section_end = lines.len();
        let mut in_section = false;

        for (index, line) in lines.iter().enumerate() {
            let (body, _) = split_eol(line);
            let trimmed = body.trim();

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                if in_section {
                    section_end = index;
                    break;
                }
                in_section = trimmed[1..trimmed.len() - 1].trim() == section;
            }
        }

        let mut insert_at = section_end;
        while insert_at > 0 {
            let (body, _) = split_eol(&lines[insert_at - 1]);
            if !body.trim().is_empty() {
                break;
            }
            insert_at -= 1;
        }

        lines.insert(insert_at, setting);
        return lines.concat();
    }
    let mut output = text.to_string();
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(newline);
    }
    output.push_str(&format!("[{}]{}{}", section, newline, setting));
    output
}

fn remove_setting(text: &str, section: &str, key: &str) -> String {
    let mut in_section = false;
    let mut removed = false;
    let mut lines = Vec::new();

    for raw in text.split_inclusive('\n') {
        let (body, _) = split_eol(raw);
        let trimmed = body.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed[1..trimmed.len() - 1].trim() == section;
        }

        if in_section {
            if let Some((name, _)) = body.split_once('=') {
                if name.trim() == key {
                    removed = true;
                    continue;
                }
            }
        }

        lines.push(raw);
    }

    if removed {
        lines.concat()
    } else {
        text.to_string()
    }
}

fn split_eol(value: &str) -> (&str, &str) {
    if let Some(body) = value.strip_suffix("\r\n") {
        (body, "\r\n")
    } else if let Some(body) = value.strip_suffix('\n') {
        (body, "\n")
    } else {
        (value, "")
    }
}

#[cfg(test)]
mod tests {
    use super::{find_setting, parse_bool, remove_setting, set_setting};

    #[test]
    fn parses_boolean_values() {
        assert!(parse_bool("true").unwrap());
        assert!(!parse_bool("FALSE").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(parse_bool("maybe").is_err());
    }

    #[test]
    fn disables_an_existing_setting_without_touching_comments() {
        let text = "[EmuCore]\nEnableDiscordPresence = true ; keep this note\nOther = 1\n";
        let updated = set_setting(text, "EmuCore", "EnableDiscordPresence", "false");
        assert!(updated.contains("EnableDiscordPresence = false ; keep this note"));
        assert!(updated.contains("Other = 1"));
        assert_eq!(
            find_setting(&updated, "EmuCore", "EnableDiscordPresence").as_deref(),
            Some("false")
        );
    }

    #[test]
    fn preserves_comment_spacing() {
        let text = "[EmuCore]\nEnableDiscordPresence = true    ; note\n";
        let updated = set_setting(text, "EmuCore", "EnableDiscordPresence", "false");
        assert!(updated.contains("EnableDiscordPresence = false    ; note"));
    }

    #[test]
    fn finds_values_without_inline_comments() {
        let text = "[EmuCore]\nEnableDiscordPresence = false ; note\n";
        assert_eq!(
            find_setting(text, "EmuCore", "EnableDiscordPresence").as_deref(),
            Some("false")
        );
    }

    #[test]
    fn adds_missing_setting_to_existing_section() {
        let text = "[EmuCore]\nOther = 1\n\n[GS]\nVSync = true\n";
        let updated = set_setting(text, "EmuCore", "EnableDiscordPresence", "false");
        assert!(updated.contains("Other = 1\nEnableDiscordPresence = false\n\n[GS]"));
    }

    #[test]
    fn adds_section_when_missing() {
        let text = "[GS]\nVSync = true\n";
        let updated = set_setting(text, "EmuCore", "EnableDiscordPresence", "false");
        assert!(updated.ends_with("[EmuCore]\nEnableDiscordPresence = false\n"));
    }

    #[test]
    fn removes_setting() {
        let text = "[EmuCore]\nEnableDiscordPresence = false\nOther = 1\n";
        let updated = remove_setting(text, "EmuCore", "EnableDiscordPresence");
        assert!(!updated.contains("EnableDiscordPresence"));
        assert!(updated.contains("Other = 1"));
    }

    #[test]
    fn preserves_the_original_value_for_restore() {
        let text = "[EmuCore]\nEnableDiscordPresence = 1\n";
        let disabled = set_setting(text, "EmuCore", "EnableDiscordPresence", "false");
        assert_eq!(
            find_setting(&disabled, "EmuCore", "EnableDiscordPresence").as_deref(),
            Some("false")
        );
        let restored = set_setting(&disabled, "EmuCore", "EnableDiscordPresence", "1");
        assert_eq!(restored, text);
    }
}
