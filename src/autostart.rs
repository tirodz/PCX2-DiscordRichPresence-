#[cfg(windows)]
mod windows {
    use std::path::Path;

    use anyhow::{Context, Result};
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    const VALUE_NAME: &str = "PCSX2 Discord Rich Presence";

    pub fn install(executable: &Path) -> Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(RUN_KEY)
            .context("opening Windows startup registry key")?;

        let command = format!("\"{}\" --background", executable.display());
        if command.len() > 260 {
            anyhow::bail!("the executable path is too long for a Windows Run entry");
        }

        key.set_value(VALUE_NAME, &command)
            .context("registering PCSX2 Discord Rich Presence at logon")?;
        Ok(())
    }

    pub fn uninstall() -> Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE) {
            Ok(key) => key,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error).context("opening Windows startup registry key"),
        };

        match key.delete_value(VALUE_NAME) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).context("removing PCSX2 Discord Rich Presence from startup"),
        }
    }

    pub fn installed() -> Result<bool> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ) {
            Ok(key) => key,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error).context("opening Windows startup registry key"),
        };

        Ok(key.get_value::<String, _>(VALUE_NAME).is_ok())
    }
}

#[cfg(not(windows))]
mod windows {
    use std::path::Path;

    use anyhow::{bail, Result};

    pub fn install(_: &Path) -> Result<()> {
        bail!("automatic startup is currently supported on Windows only")
    }

    pub fn uninstall() -> Result<()> {
        bail!("automatic startup is currently supported on Windows only")
    }

    pub fn installed() -> Result<bool> {
        Ok(false)
    }
}

pub fn install() -> anyhow::Result<()> {
    let executable = std::env::current_exe()?;
    windows::install(&executable)
}

pub fn uninstall() -> anyhow::Result<()> {
    windows::uninstall()?;
    request_stop()?;

    let pid_path = control_path(".pid")?;
    if pid_path.exists() {
        for _ in 0..40 {
            if !pid_path.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }

    Ok(())
}

fn control_path(name: &str) -> anyhow::Result<std::path::PathBuf> {
    Ok(std::env::current_exe()?
        .parent()
        .map(|p| p.join(name))
        .ok_or_else(|| anyhow::anyhow!("executable directory is unavailable"))?)
}

pub fn request_stop() -> anyhow::Result<()> {
    std::fs::write(control_path(".stop")?, b"stop")?;
    Ok(())
}

pub fn write_pid() -> anyhow::Result<()> {
    std::fs::write(control_path(".pid")?, std::process::id().to_string())?;
    Ok(())
}

pub fn clear_pid() -> anyhow::Result<()> {
    match std::fs::remove_file(control_path(".pid")?) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub fn clear_stop_request() -> anyhow::Result<()> {
    match std::fs::remove_file(control_path(".stop")?) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub fn stop_requested() -> anyhow::Result<bool> {
    Ok(control_path(".stop")?.exists())
}

pub fn installed() -> anyhow::Result<bool> {
    windows::installed()
}
