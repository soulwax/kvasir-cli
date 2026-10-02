use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "config.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub arl: String,
    #[serde(default)]
    pub tidal_access_token: String,
    #[serde(default)]
    pub tidal_refresh_token: String,
    #[serde(default = "default_quality")]
    pub quality: String,
    #[serde(default)]
    pub download_dir: PathBuf,
}

fn default_quality() -> String {
    "flac".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            arl: String::new(),
            tidal_access_token: String::new(),
            tidal_refresh_token: String::new(),
            quality: default_quality(),
            download_dir: default_download_dir(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("KVASIR_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    directories::BaseDirs::new()
        .map(|dirs| dirs.config_dir().join("kvasir"))
        .unwrap_or_else(|| PathBuf::from("kvasir"))
}

fn default_download_dir() -> PathBuf {
    directories::UserDirs::new()
        .and_then(|dirs| dirs.audio_dir().map(|dir| dir.join("Kvasir")))
        .unwrap_or_else(|| config_dir().join("music"))
}

impl Config {
    pub fn load() -> Self {
        let path = config_dir().join(FILE_NAME);
        let Ok(text) = fs::read_to_string(&path) else {
            return Self::default();
        };
        let mut config: Self = serde_json::from_str(&text).unwrap_or_default();
        if config.quality.trim().is_empty() {
            config.quality = default_quality();
        }
        if config.download_dir.as_os_str().is_empty() {
            config.download_dir = default_download_dir();
        }
        config
    }

    pub fn save(&self) -> std::io::Result<PathBuf> {
        let dir = config_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(FILE_NAME);
        let text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        fs::write(&path, text)?;
        restrict(&path);
        Ok(path)
    }

    pub fn deezer_signed_in(&self) -> bool {
        self.arl.trim().len() == 192
    }

    pub fn tidal_signed_in(&self) -> bool {
        !self.tidal_access_token.trim().is_empty()
    }
}

pub fn reset() -> std::io::Result<PathBuf> {
    let dir = config_dir();
    ensure_reset_target(&dir)?;
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(dir)
}

fn ensure_reset_target(dir: &Path) -> std::io::Result<()> {
    let name = dir.file_name().and_then(|name| name.to_str());
    if name != Some("kvasir") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("refusing to delete {}", dir.display()),
        ));
    }
    if dir.components().count() < 2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("refusing to delete {}", dir.display()),
        ));
    }
    Ok(())
}

fn restrict(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn save_load_and_reset_round_trip() {
        let _guard = LOCK.lock().expect("lock");
        let dir = std::env::temp_dir()
            .join(format!("kvasir-config-test-{}", std::process::id()))
            .join("kvasir");
        let _ = fs::remove_dir_all(&dir);
        std::env::set_var("KVASIR_CONFIG_DIR", &dir);
        let mut config = Config::default();
        config.arl = "a".repeat(192);
        config.quality = "320".into();
        config.download_dir = dir.join("out");
        config.save().expect("save");
        let loaded = Config::load();
        assert_eq!(loaded.arl.len(), 192);
        assert_eq!(loaded.quality, "320");
        assert!(loaded.deezer_signed_in());
        let removed = reset().expect("reset");
        assert_eq!(removed, dir);
        assert!(!dir.exists());
        std::env::remove_var("KVASIR_CONFIG_DIR");
    }

    #[test]
    fn reset_refuses_a_folder_not_named_kvasir() {
        let err = ensure_reset_target(Path::new("/tmp/other")).unwrap_err();
        assert!(err.to_string().contains("refusing"));
    }

    #[cfg(windows)]
    #[test]
    fn default_dir_is_roaming_appdata() {
        let _guard = LOCK.lock().expect("lock");
        std::env::remove_var("KVASIR_CONFIG_DIR");
        let dir = config_dir();
        let text = dir.to_string_lossy().to_ascii_lowercase().replace('/', "\\");
        assert!(
            text.contains("\\appdata\\roaming\\kvasir"),
            "config dir was {text}"
        );
    }
}
